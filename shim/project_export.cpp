#include <uapmd-data/uapmd-data.hpp>
#include <choc/text/choc_JSON.h>
#include <stdexcept>
#include <algorithm>

namespace {
// Separate UMP groups let each instrument receive its own already-routed notes,
// while their audio can merge before a shared, potentially nonlinear effect.
void groupedClip(const std::filesystem::path& root, const choc::value::ValueView& files,
                 const std::filesystem::path& output, uint32_t division) {
    struct Event { uint64_t tick; umppi::Ump ump; };
    std::vector<Event> events;
    uint32_t group = 0;
    for (auto file : files) {
        auto info = uapmd::MidiClipReader::readAnyFormat(root / std::string(file.getString()));
        if (!info.success) throw std::runtime_error(info.error);
        for (size_t offset = 0; offset < info.ump_data.size();) {
            umppi::Ump ump(info.ump_data[offset],
                offset + 1 < info.ump_data.size() ? info.ump_data[offset + 1] : 0,
                offset + 2 < info.ump_data.size() ? info.ump_data[offset + 2] : 0,
                offset + 3 < info.ump_data.size() ? info.ump_data[offset + 3] : 0);
            auto count = ump.getSizeInBytes() / 4;
            if (count <= 0 || offset + count > info.ump_data.size())
                throw std::runtime_error("Invalid project UMP length");
            ump.int1 = (ump.int1 & 0xf0ffffffu) | (group << 24);
            events.push_back({info.ump_tick_timestamps.at(offset), ump});
            offset += count;
        }
        ++group;
    }
    std::stable_sort(events.begin(), events.end(), [](const auto& a, const auto& b) { return a.tick < b.tick; });
    uapmd::Smf2Clip clip{umppi::Ump(umppi::UmpFactory::deltaClockstamp(0)),
        umppi::Ump(umppi::UmpFactory::dctpq(division)),
        umppi::Ump(umppi::UmpFactory::deltaClockstamp(0)), umppi::UmpFactory::startOfClip()};
    uint64_t previous = 0;
    for (const auto& event : events) {
        auto delta = event.tick - previous;
        if (delta > 0xfffff) throw std::runtime_error("Project MIDI event gap exceeds Delta Clockstamp range");
        clip.emplace_back(umppi::UmpFactory::deltaClockstamp(static_cast<uint32_t>(delta)));
        clip.push_back(event.ump);
        previous = event.tick;
    }
    clip.emplace_back(umppi::UmpFactory::deltaClockstamp(0));
    clip.push_back(umppi::UmpFactory::endOfClip());
    std::string error;
    if (!uapmd::Smf2ClipReaderWriter::write(output, clip, &error)) throw std::runtime_error(error);
}
}

// Clips, graphs and opaque states are staged by Rust. Keep project serialization
// owned by UAPMD, just as import uses its reader.
extern "C" __declspec(dllexport) const char* smfe_write_project(const char* directory, const char* manifest) noexcept {
    static thread_local std::string result;
    try {
        auto root = std::filesystem::u8path(directory);
        auto input = choc::json::parse(manifest);
        auto project = uapmd::UapmdProjectData::create();
        auto clip = [&](uapmd::UapmdProjectTrackData* track, std::string file) {
            auto c = uapmd::UapmdProjectClipData::create();
            c->position({});
            c->file(file);
            c->clipType("midi");
            c->tickResolution(input["division"].get<int>());
            track->clips().push_back(std::move(c));
        };
        project->masterTrack()->volume(1.0);
        if (input["master_effects"].get<bool>()) {
            auto graph = uapmd::UapmdProjectPluginGraphData::create();
            graph->graphType("urn:uapmd-graph:common/graph/dag/v1");
            graph->externalFile("graphs/master.json");
            project->masterTrack()->graph(std::move(graph));
        }
        clip(project->masterTrack(), "clips/master.mid");
        int index = 0;
        for (auto item : input["tracks"]) {
            auto track = uapmd::UapmdProjectTrackData::create();
            track->volume(item["volume"].get<double>());
            // Layer solo has already been resolved within its source track.
            track->solo(false);
            auto graph = uapmd::UapmdProjectPluginGraphData::create();
            graph->graphType("urn:uapmd-graph:common/graph/dag/v1");
            graph->externalFile("graphs/" + std::to_string(index) + ".json");
            track->graph(std::move(graph));
            if (item["clips"].size() > 1) {
                auto file = "clips/" + std::to_string(index) + ".midi2";
                groupedClip(root, item["clips"], root / file, input["division"].get<uint32_t>());
                clip(track.get(), file);
            } else {
                clip(track.get(), std::string(item["clips"][0].getString()));
            }
            ++index;
            project->addTrack(std::move(track));
        }
        if (!uapmd::UapmdProjectDataWriter::write(project.get(), root / "project.uapmd"))
            throw std::runtime_error("Could not write UAPMD project");
        result.clear();
    } catch (const std::exception& e) { result = e.what(); }
      catch (...) { result = "Unknown project export failure"; }
    return result.c_str();
}
