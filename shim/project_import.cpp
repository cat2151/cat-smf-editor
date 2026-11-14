#include <uapmd-data/uapmd-data.hpp>
#include <choc/text/choc_JSON.h>
#include <cmath>
#include <stdexcept>

namespace {
using choc::value::Value;
namespace fs = std::filesystem;

fs::path contained(const fs::path& root, const fs::path& relative) {
    if (relative.empty() || relative.is_absolute())
        throw std::runtime_error("Project reference must be relative");
    auto path = fs::canonical(root / relative);
    auto rel = path.lexically_relative(fs::canonical(root));
    if (rel.empty() || *rel.begin() == "..")
        throw std::runtime_error("Project reference escapes the archive");
    return path;
}

Value event(uint64_t tick, uint8_t status, uint8_t meta, std::initializer_list<uint8_t> data) {
    auto bytes = choc::value::createEmptyArray();
    for (auto b : data) bytes.addArrayElement(static_cast<int>(b));
    return choc::value::createObject("", "tick", static_cast<int64_t>(tick),
        "status", static_cast<int>(status), "meta_type", static_cast<int>(meta), "data", bytes);
}

void readClips(uapmd::UapmdProjectTrackData& track, const fs::path& root, bool master,
               uint32_t& division, Value& events) {
    if (!master && track.clips().size() != 1)
        throw std::runtime_error("Only one MIDI clip per instrument track is supported");
    for (auto& clip : track.clips()) {
        auto pos = clip->position();
        if (pos.anchor || pos.samples || clip->sourceOffsetSamples())
            throw std::runtime_error("Only untrimmed clips at project start are supported");
        if (clip->clipType() != "midi")
            throw std::runtime_error("Only MIDI clips are supported");
        auto info = uapmd::MidiClipReader::readAnyFormat(contained(root, clip->file()));
        if (!info.success) throw std::runtime_error(info.error);
        if (!info.tick_resolution || info.tick_resolution > 32767)
            throw std::runtime_error("Unsupported MIDI tick resolution");
        if (division && division != info.tick_resolution)
            throw std::runtime_error("Mixed MIDI tick resolutions are not supported");
        division = info.tick_resolution;
        if (master) {
            if (info.has_explicit_tempo_changes) for (auto& tempo : info.tempo_changes) {
                auto us = std::llround(60000000.0 / tempo.bpm);
                if (us <= 0 || us > 0xffffff) throw std::runtime_error("Invalid MIDI tempo");
                events.addArrayElement(event(tempo.tickPosition, 0xff, 0x51,
                    {static_cast<uint8_t>(us >> 16), static_cast<uint8_t>(us >> 8), static_cast<uint8_t>(us)}));
            }
            if (info.has_explicit_time_signature_changes) for (auto& sig : info.time_signature_changes) {
                unsigned denom = sig.denominator, power = 0;
                while (denom > 1 && !(denom & 1)) { denom >>= 1; ++power; }
                if (denom != 1) throw std::runtime_error("Invalid time signature");
                events.addArrayElement(event(sig.tickPosition, 0xff, 0x58,
                    {sig.numerator, static_cast<uint8_t>(power), 24, 8}));
            }
        } else if (info.has_explicit_tempo_changes || info.has_explicit_time_signature_changes) {
            throw std::runtime_error("Tempo and time signature must belong to the master track");
        }
        for (size_t offset = 0; offset < info.ump_data.size();) {
            umppi::Ump ump(info.ump_data[offset],
                offset + 1 < info.ump_data.size() ? info.ump_data[offset + 1] : 0,
                offset + 2 < info.ump_data.size() ? info.ump_data[offset + 2] : 0,
                offset + 3 < info.ump_data.size() ? info.ump_data[offset + 3] : 0);
            const auto count = ump.getSizeInBytes() / 4;
            if (count <= 0 || offset + count > info.ump_data.size())
                throw std::runtime_error("Invalid UMP length");
            const auto tick = info.ump_tick_timestamps.at(offset);
            offset += count;
            // Generated clip names are metadata, and must not become timestamp text in the SMF.
            if (ump.getMessageType() == umppi::MessageType::FLEX_DATA &&
                ((ump.int1 >> 8) & 255) == umppi::FlexDataStatusBank::METADATA_TEXT &&
                (ump.int1 & 255) == umppi::MetadataTextStatus::MIDI_CLIP_NAME) continue;
            if (master || (ump.getMessageType() != umppi::MessageType::MIDI1 &&
                           ump.getMessageType() != umppi::MessageType::MIDI2) ||
                (ump.getStatusCode() != umppi::MidiChannelStatus::NOTE_ON &&
                 ump.getStatusCode() != umppi::MidiChannelStatus::NOTE_OFF))
                throw std::runtime_error("Only note events and master tempo/time signature are supported");
            if (ump.getGroup() != 0)
                throw std::runtime_error("Only UMP group 0 is supported");
            std::vector<umppi::Ump> translated;
            umppi::UmpTranslator::translateMidi2UmpToMidi1Ump(translated, {ump});
            for (auto& m : translated)
                events.addArrayElement(event(tick, m.getStatusByte(), 0, {m.getMidi1Msb(), m.getMidi1Lsb()}));
        }
    }
}
}

extern "C" __declspec(dllexport) const char* smfe_read_project(const char* path) noexcept {
    static thread_local std::string result;
    try {
        auto file = fs::u8path(path);
        auto project = uapmd::UapmdProjectDataReader::read(file);
        if (!project || project->tracks().empty()) throw std::runtime_error("Project has no instrument tracks");
        uint32_t division = 0;
        auto master = choc::value::createEmptyArray();
        auto masterTrack = project->masterTrack();
        if (!masterTrack) throw std::runtime_error("Project has no master track");
        if (masterTrack->muted() || masterTrack->solo() || masterTrack->volume() != 1.0)
            throw std::runtime_error("Only an unmuted unity-gain master without solo is supported");
        readClips(*masterTrack, file.parent_path(), true, division, master);
        auto tracks = choc::value::createEmptyArray();
        for (auto track : project->tracks()) {
            if (track->muted() || track->solo() || track->volume() != 1.0)
                throw std::runtime_error("Only unmuted unity-gain tracks without solo are supported");
            auto events = choc::value::createEmptyArray();
            readClips(*track, file.parent_path(), false, division, events);
            if (!track->graph() || track->graph()->externalFile().empty())
                throw std::runtime_error("An external instrument graph is required");
            tracks.addArrayElement(choc::value::createObject("", "graph", track->graph()->externalFile().generic_string(), "events", events));
        }
        result = choc::json::toString(choc::value::createObject("", "division", static_cast<int>(division),
            "master", master, "master_graph", masterTrack->graph() ? masterTrack->graph()->externalFile().generic_string() : "",
            "tracks", tracks));
    } catch (const std::exception& error) {
        result = choc::json::toString(choc::value::createObject("", "error", error.what()));
    } catch (...) {
        result = "{\"error\":\"Unknown project import failure\"}";
    }
    return result.c_str();
}
