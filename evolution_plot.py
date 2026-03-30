import json
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt
import seaborn as sns

SUMMARY_FILE = "summary_data.jsonl"
WORLD_FILE = "world_data.jsonl"

BIOME_NAMES = ["Forest", "Desert", "Water", "Grassland"]
BIOME_IDX = {name: i for i, name in enumerate(BIOME_NAMES)}
BIOME_COLORS = {0: "green", 1: "yellow", 2: "blue", 3: "brown"}

# ---------------------------------------------------------------------------
# 1. Time-series stats from summary_data.jsonl (lightweight, pre-aggregated)
# ---------------------------------------------------------------------------
gen_list = []
organism_counts = []
predator_counts = []
organism_avg_size_list = []
organism_avg_speed_list = []
organism_avg_energy_list = []
organism_avg_reproduction_threshold_list = []
predator_avg_size_list = []
predator_avg_speed_list = []
predator_avg_energy_list = []
predator_avg_reproduction_threshold_list = []
predator_avg_hunting_efficiency_list = []
predator_avg_satiation_threshold_list = []
average_food_per_generation = []
# biome_tally values are avg biome tolerance sums per generation
biome_tolerance_avg = {name: [] for name in BIOME_NAMES}

with open(SUMMARY_FILE) as f:
    for line in f:
        if not line.strip():
            continue
        d = json.loads(line)
        gen_list.append(d["generation"])
        organism_counts.append(d["organism_count"])
        predator_counts.append(d["predator_count"])
        organism_avg_size_list.append(d["organism_avg_size"])
        organism_avg_speed_list.append(d["organism_avg_speed"])
        organism_avg_energy_list.append(max(d["organism_avg_energy"], 0))
        organism_avg_reproduction_threshold_list.append(d["organism_avg_reproduction_threshold"])
        predator_avg_size_list.append(d["predator_avg_size"])
        predator_avg_speed_list.append(d["predator_avg_speed"])
        predator_avg_energy_list.append(max(d["predator_avg_energy"], 0))
        predator_avg_reproduction_threshold_list.append(d["predator_avg_reproduction_threshold"])
        predator_avg_hunting_efficiency_list.append(d["predator_avg_hunting_efficiency"])
        predator_avg_satiation_threshold_list.append(d["predator_avg_satiation_threshold"])
        average_food_per_generation.append(d["average_food"])
        tally = d["biome_tally"]
        for name in BIOME_NAMES:
            biome_tolerance_avg[name].append(tally.get(name, 0.0))

print(f"Loaded {len(gen_list)} generations from {SUMMARY_FILE}")

# ---------------------------------------------------------------------------
# 2. Spatial data from world_data.jsonl (biome map, heatmaps)
# ---------------------------------------------------------------------------
width = height = None
heatmap_grid = None
last_food = None
world_biome_grid = None
lines_processed = 0

with open(WORLD_FILE) as f:
    for line in f:
        if not line.strip():
            continue
        d = json.loads(line)

        if width is None:
            width = d["config"]["width"]
            height = d["config"]["height"]
            heatmap_grid = np.zeros((height, width))
            flat_tiles = d["world"]["grid"]  # flat Vec<Tile>, row-major y*width+x
            world_biome_grid = np.array(
                [BIOME_IDX[t["biome"]] for t in flat_tiles], dtype=int
            ).reshape(height, width)

        for org in d["organisms"]:
            heatmap_grid[org["position"]["y"], org["position"]["x"]] += 1
        for pred in d["predators"]:
            heatmap_grid[pred["position"]["y"], pred["position"]["x"]] += 1

        last_food = d["food"]  # flat [f32] array, same row-major layout
        lines_processed += 1
        if lines_processed % 100 == 0:
            print(f"  world entries processed: {lines_processed}")

print(f"Loaded {lines_processed} world snapshots from {WORLD_FILE}")

# ---------------------------------------------------------------------------
# 3. Plots
# ---------------------------------------------------------------------------

# Biome map
fig, ax = plt.subplots(figsize=(max(width / 10, 4), max(height / 10, 4)))
for y in range(height):
    for x in range(width):
        color = BIOME_COLORS[world_biome_grid[y, x]]
        ax.add_patch(plt.Rectangle((x, y), 1, 1, color=color))
ax.set_xlim(0, width)
ax.set_ylim(0, height)
ax.set_aspect("equal")
ax.set_title("World Biome Map")
ax.set_xlabel("X Position")
ax.set_ylabel("Y Position")
ax.legend(
    handles=[
        plt.Line2D([0], [0], marker="o", color="w", label=name, markerfacecolor=BIOME_COLORS[i])
        for name, i in BIOME_IDX.items()
    ],
    loc="upper right",
)
plt.tight_layout()
plt.savefig("world_biome_map.png")
plt.close()

# Population over time
plt.figure(figsize=(10, 5))
plt.plot(gen_list, organism_counts, label="Preys", color="lime", linewidth=2)
plt.plot(gen_list, predator_counts, label="Predators", color="red", linewidth=2)
plt.xlabel("Generation")
plt.ylabel("Population")
plt.title("Population Over Time")
plt.legend()
plt.grid(True)
plt.savefig("population_trends.png")
plt.close()

# Population heatmap
plt.figure(figsize=(8, 6))
sns.heatmap(heatmap_grid, cmap="hot", square=True)
plt.title("Population Heatmap (cumulative)")
plt.xlabel("X")
plt.ylabel("Y")
plt.savefig("population_heatmap.png")
plt.close()

# Energy distribution
plt.figure(figsize=(10, 5))
sns.histplot(organism_avg_energy_list, bins=30, color="lime", alpha=0.7, label="Preys", kde=True)
sns.histplot(predator_avg_energy_list, bins=30, color="red", alpha=0.7, label="Predators", kde=True)
plt.xlabel("Avg Energy")
plt.ylabel("Frequency")
plt.title("Energy Distribution of Preys and Predators")
plt.legend()
plt.grid(True)
plt.savefig("energy_distribution.png")
plt.close()

# Biome tolerance trends (avg tolerance sum per biome per generation)
df_biomes = pd.DataFrame(biome_tolerance_avg, index=gen_list)
df_biomes.plot(kind="area", stacked=True, figsize=(10, 6), colormap="coolwarm", alpha=0.7)
plt.xlabel("Generation")
plt.ylabel("Avg Biome Tolerance Sum")
plt.title("Biome Tolerance Trends Over Generations")
plt.legend(title="Biome")
plt.grid(True)
plt.savefig("biome_trends.png")
plt.close()

# Food availability over time
plt.figure(figsize=(10, 5))
plt.plot(gen_list, average_food_per_generation, label="Avg Food", color="orange", linewidth=2)
plt.xlabel("Generation")
plt.ylabel("Average Food Availability")
plt.title("Food Availability Trends Over Generations")
plt.legend()
plt.grid(True)
plt.savefig("food_trends.png")
plt.close()

# Food heatmap (last snapshot)
food_grid = np.array(last_food).reshape(height, width)
plt.figure(figsize=(8, 6))
sns.heatmap(food_grid, cmap="YlGnBu", square=True)
plt.title(f"Food Availability Heatmap (Generation {gen_list[-1]})")
plt.xlabel("X Position")
plt.ylabel("Y Position")
plt.savefig("food_heatmap.png")
plt.close()

# Traits evolution
fig, axes = plt.subplots(3, 1, figsize=(10, 12))
axes[0].plot(gen_list, organism_avg_size_list, label="Preys - Avg Size", color="lime", linewidth=2)
axes[0].plot(gen_list, predator_avg_size_list, label="Predators - Avg Size", color="red", linewidth=2)
axes[0].set_ylabel("Size")
axes[0].set_title("Evolution of Size Over Generations")
axes[0].legend()
axes[0].grid(True)

axes[1].plot(gen_list, organism_avg_speed_list, label="Preys - Avg Speed", color="blue", linewidth=2)
axes[1].plot(gen_list, predator_avg_speed_list, label="Predators - Avg Speed", color="orange", linewidth=2)
axes[1].set_ylabel("Speed")
axes[1].set_title("Evolution of Speed Over Generations")
axes[1].legend()
axes[1].grid(True)

axes[2].plot(gen_list, organism_avg_energy_list, label="Preys - Avg Energy", color="yellow", linewidth=2)
axes[2].plot(gen_list, predator_avg_energy_list, label="Predators - Avg Energy", color="blue", linewidth=2)
axes[2].set_xlabel("Generation")
axes[2].set_ylabel("Energy")
axes[2].set_title("Evolution of Energy Over Generations")
axes[2].legend()
axes[2].grid(True)

plt.tight_layout()
plt.savefig("traits_evolution.png")
plt.close()

# Reproduction threshold
plt.figure(figsize=(10, 5))
plt.plot(gen_list, organism_avg_reproduction_threshold_list, label="Preys", color="lime", linewidth=2)
plt.plot(gen_list, predator_avg_reproduction_threshold_list, label="Predators", color="red", linewidth=2)
plt.xlabel("Generation")
plt.ylabel("Reproduction Threshold")
plt.title("Reproduction Threshold Evolution Over Generations")
plt.legend()
plt.grid(True)
plt.savefig("reproduction_threshold_trends.png")
plt.close()

# Predator hunting efficiency
plt.figure(figsize=(10, 5))
plt.plot(gen_list, predator_avg_hunting_efficiency_list, label="Predators - Hunting Efficiency", color="blue", linewidth=2)
plt.xlabel("Generation")
plt.ylabel("Hunting Efficiency")
plt.title("Predator Hunting Efficiency Over Generations")
plt.legend()
plt.grid(True)
plt.savefig("hunting_efficiency_trends.png")
plt.close()

df = pd.DataFrame({
    "Generation": gen_list,
    "Organism Size": organism_avg_size_list,
    "Predator Size": predator_avg_size_list,
    "Organism Speed": organism_avg_speed_list,
    "Predator Speed": predator_avg_speed_list,
    "Organism Energy": organism_avg_energy_list,
    "Predator Energy": predator_avg_energy_list,
    "Organism Reproduction Threshold": organism_avg_reproduction_threshold_list,
    "Predator Hunting Efficiency": predator_avg_hunting_efficiency_list,
})
print("DataFrame head:\n", df.head())
print("Done. Plots saved.")
