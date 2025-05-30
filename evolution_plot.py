import json
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt
import seaborn as sns
from scipy.stats import pearsonr

jsonl_file = "world_data.jsonl"
generations = []
organism_count = 0
predator_count = 0
width, height = None, None

gen_list = []
organism_counts = []
predator_counts = []
biome_counts = {"Forest": [], "Desert": [], "Water": [], "Grassland": []}

heatmap_grid = None
last_snapshot = None

organism_avg_size_list = []
organism_avg_speed_list = []
organism_avg_energy_list = []
predator_avg_size_list = []
predator_avg_speed_list = []
predator_avg_energy_list = []
organism_avg_reproduction_threshold_list = []
predator_avg_reproduction_threshold_list = []
predator_avg_hunting_efficiency_list = []
predator_avg_satiation_threshold_list = []

average_food_per_generation = []

BIOME_STR_TO_ID = {
    "Forest": 0,
    "Desert": 1,
    "Water": 2,
    "Grassland": 3,
}

BIOME_COLORS = {
    0: "green",
    1: "yellow",
    2: "blue",
    3: "brown",
}

# BiomeMap holds data about world (pos and biome) so later i can display how world looks
class BiomeMap:
    def __init__(self, width, height):
        self.width = width
        self.height = height
        self.grid = np.zeros((height, width), dtype=int)

    def update(self, x, y, biome):
        self.grid[y, x] = biome

    def get_biome(self, x, y):
        return self.grid[y, x]
    
    def plot(self):
        """Draw the world with biomes, each cell is a biome and color is based on biome. Cell size is 10x10.
        Put rectangles in the grid and color them based on biome.
        """
        fig, ax = plt.subplots(figsize=(self.width / 10, self.height / 10))
        for y in range(self.height):
            for x in range(self.width):
                biome = self.get_biome(x, y)
                color = BIOME_COLORS[biome]
                rect = plt.Rectangle((x, y), 1, 1, color=color)
                ax.add_patch(rect)
        ax.set_xlim(0, self.width)
        ax.set_ylim(0, self.height)
        ax.set_aspect("equal")
        plt.title("World Biome Map")
        plt.xlabel("X Position")
        plt.ylabel("Y Position")
        plt.legend(
            handles=[
                plt.Line2D([0], [0], marker="o", color="w", label="Forest", markerfacecolor="green"),
                plt.Line2D([0], [0], marker="o", color="w", label="Desert", markerfacecolor="yellow"),
                plt.Line2D([0], [0], marker="o", color="w", label="Water", markerfacecolor="blue"),
                plt.Line2D([0], [0], marker="o", color="w", label="Grassland", markerfacecolor="brown"),
            ],
            loc="upper right",
        )
        plt.savefig("world_biome_map.png")
        plt.close()
    
world_biome_map = None

with open(jsonl_file, "r") as f:
    for line in f:
        if not line.strip():
            continue
        data = json.loads(line.strip())
        last_snapshot = data

        if not world_biome_map:
            world_biome_map = BiomeMap(data["config"]["width"], data["config"]["height"])
            for y in range(data["config"]["height"]):
                for x in range(data["config"]["width"]):
                    biome = data["world"]["grid"][y][x]["biome"]
                    biome_id = BIOME_STR_TO_ID[biome]
                    world_biome_map.update(x, y, biome_id)
            world_biome_map.plot()

        generation = data["generation"]
        generations.append(generation)
        organism_count = len(data["organisms"])
        predator_count = len(data["predators"])

        if width is None:
            width, height = data["config"]["width"], data["config"]["height"]
            heatmap_grid = np.zeros((height, width))

        gen_list.append(generation)
        organism_counts.append(organism_count)
        predator_counts.append(predator_count)

        if organism_count > 0:
            organism_avg_size_list.append(
                np.mean([o["organism"]["size"] for o in data["organisms"]])
            )
            organism_avg_speed_list.append(
                np.mean([o["organism"]["speed"] for o in data["organisms"]])
            )
            organism_avg_energy_list.append(
                np.mean([o["organism"]["energy"] for o in data["organisms"]])
            )
            organism_avg_reproduction_threshold_list.append(
                np.mean(
                    [o["organism"]["reproduction_threshold"] for o in data["organisms"]]
                )
            )
        else:
            organism_avg_size_list.append(0)
            organism_avg_speed_list.append(0)
            organism_avg_energy_list.append(0)
            organism_avg_reproduction_threshold_list.append(0)

        if predator_count > 0:
            predator_avg_size_list.append(
                np.mean([p["predator"]["size"] for p in data["predators"]])
            )
            predator_avg_speed_list.append(
                np.mean([p["predator"]["speed"] for p in data["predators"]])
            )
            predator_avg_energy_list.append(
                np.mean([p["predator"]["energy"] for p in data["predators"]])
            )
            predator_avg_reproduction_threshold_list.append(
                np.mean(
                    [p["predator"]["reproduction_threshold"] for p in data["predators"]]
                )
            )
            predator_avg_hunting_efficiency_list.append(
                np.mean(
                    [p["predator"]["hunting_efficiency"] for p in data["predators"]]
                )
            )
            predator_avg_satiation_threshold_list.append(
                np.mean(
                    [p["predator"]["satiation_threshold"] for p in data["predators"]]
                )
            )
        else:
            predator_avg_size_list.append(0)
            predator_avg_speed_list.append(0)
            predator_avg_energy_list.append(0)
            predator_avg_reproduction_threshold_list.append(0)
            predator_avg_hunting_efficiency_list.append(0)
            predator_avg_satiation_threshold_list.append(0)

        for org in data["organisms"]:
            x, y = org["position"]["x"], org["position"]["y"]
            heatmap_grid[y, x] += 1
        for pred in data["predators"]:
            x, y = pred["position"]["x"], pred["position"]["y"]
            heatmap_grid[y, x] += 1

        biome_tally = {"Forest": 0, "Desert": 0, "Water": 0, "Grassland": 0}
        for org in data["organisms"]:
            max_biome = max(
                org["organism"]["biome_tolerance"],
                key=org["organism"]["biome_tolerance"].get,
            )
            biome_tally[max_biome] += 1
        for biome in biome_counts:
            biome_counts[biome].append(biome_tally[biome])

        total_food = sum(
            tile["food_availabilty"] for row in data["world"]["grid"] for tile in row
        )
        average_food_per_generation.append(total_food / (width * height))

        if len(gen_list) % 100 == 0:
            print(f"Processed {len(gen_list)} generations...")

organism_avg_energy_list = [max(x, 0) for x in organism_avg_energy_list]
predator_avg_energy_list = [max(x, 0) for x in predator_avg_energy_list]

plt.figure(figsize=(10, 5))
plt.plot(gen_list, organism_counts, label="Preys", color="lime", linewidth=2)
plt.plot(gen_list, predator_counts, label="Predators", color="red", linewidth=2)
plt.xlabel("Generation")
plt.ylabel("Population")
plt.title("Population Over Time")
plt.legend()
plt.grid(True)
plt.savefig("population_trends.png")

plt.figure(figsize=(8, 6))
sns.heatmap(heatmap_grid, cmap="hot", square=True)
plt.title("Population Heatmap")
plt.xlabel("X")
plt.ylabel("Y")
plt.savefig("population_heatmap.png")

plt.figure(figsize=(10, 5))
sns.histplot(
    organism_avg_energy_list,
    bins=30,
    color="lime",
    alpha=0.7,
    label="Preys",
    kde=True,
)
sns.histplot(
    predator_avg_energy_list,
    bins=30,
    color="red",
    alpha=0.7,
    label="Predators",
    kde=True,
)
plt.xlabel("Energy Levels")
plt.ylabel("Frequency")
plt.title("Energy Distribution of Preys and Predators")
plt.legend()
plt.grid(True)
plt.savefig("energy_distribution.png")


df_biomes = pd.DataFrame(biome_counts, index=generations)
df_biomes.plot(
    kind="area", stacked=True, figsize=(10, 6), colormap="coolwarm", alpha=0.7
)
plt.xlabel("Generation")
plt.ylabel("Organism Count")
plt.title("Biome Preference Trends Over Generations")
plt.legend(title="Biome")
plt.grid(True)
plt.savefig("biome_trends.png")

plt.figure(figsize=(10, 5))
plt.plot(
    gen_list,
    average_food_per_generation,
    label="Avg Food Availability",
    color="orange",
    linewidth=2,
)
plt.xlabel("Generation")
plt.ylabel("Average Food Availability")
plt.title("Food Availability Trends Over Generations")
plt.legend()
plt.grid(True)
plt.savefig("food_trends.png")

food_grid = np.array(
    [
        [tile["food_availabilty"] for tile in row]
        for row in last_snapshot["world"]["grid"]
    ]
)

plt.figure(figsize=(8, 6))
sns.heatmap(food_grid, cmap="YlGnBu", square=True)
plt.title(f"Food Availability Heatmap (Generation {last_snapshot['generation']})")
plt.xlabel("X Position")
plt.ylabel("Y Position")
plt.savefig("food_heatmap.png")

fig, axes = plt.subplots(3, 1, figsize=(10, 12))
axes[0].plot(
    gen_list,
    organism_avg_size_list,
    label="Preys - Avg Size",
    color="lime",
    linewidth=2,
)
axes[0].plot(
    gen_list,
    predator_avg_size_list,
    label="Predators - Avg Size",
    color="red",
    linewidth=2,
)
axes[0].set_ylabel("Size")
axes[0].set_title("Evolution of Size Over Generations")
axes[0].legend()
axes[0].grid(True)

axes[1].plot(
    gen_list,
    organism_avg_speed_list,
    label="Preys - Avg Speed",
    color="blue",
    linewidth=2,
)
axes[1].plot(
    gen_list,
    predator_avg_speed_list,
    label="Predators - Avg Speed",
    color="orange",
    linewidth=2,
)
axes[1].set_ylabel("Speed")
axes[1].set_title("Evolution of Speed Over Generations")
axes[1].legend()
axes[1].grid(True)

axes[2].plot(
    gen_list,
    organism_avg_energy_list,
    label="Preys - Avg Energy",
    color="yellow",
    linewidth=2,
)
axes[2].plot(
    gen_list,
    predator_avg_energy_list,
    label="Predators - Avg Energy",
    color="blue",
    linewidth=2,
)
axes[2].set_xlabel("Generation")
axes[2].set_ylabel("Energy")
axes[2].set_title("Evolution of Energy Over Generations")
axes[2].legend()
axes[2].grid(True)

plt.tight_layout()
plt.savefig("traits_evolution.png")

df = pd.DataFrame(
    {
        "Generation": generations,
        "Organism Size": organism_avg_size_list,
        "Predator Size": predator_avg_size_list,
        "Organism Speed": organism_avg_speed_list,
        "Predator Speed": predator_avg_speed_list,
        "Organism Energy": organism_avg_energy_list,
        "Predator Energy": predator_avg_energy_list,
        "Organism Reproduction Threshold": organism_avg_reproduction_threshold_list,
        "Predator Hunting Efficiency": predator_avg_hunting_efficiency_list,
    }
)

print("DataFrame head:\n", df.head())

plt.figure(figsize=(10, 5))
plt.plot(
    gen_list,
    organism_avg_reproduction_threshold_list,
    label="Preys - Reproduction Threshold",
    color="lime",
    linewidth=2,
)
plt.plot(
    gen_list,
    predator_avg_reproduction_threshold_list,
    label="Predators - Reproduction Threshold",
    color="red",
    linewidth=2,
)
plt.xlabel("Generation")
plt.ylabel("Reproduction Threshold")
plt.title("Reproduction Threshold Evolution Over Generations")
plt.legend()
plt.grid(True)
plt.savefig("reproduction_threshold_trends.png")

plt.figure(figsize=(10, 5))
plt.plot(
    gen_list,
    predator_avg_hunting_efficiency_list,
    label="Predators - Hunting Efficiency",
    color="blue",
    linewidth=2,
)
plt.xlabel("Generation")
plt.ylabel("Hunting Efficiency")
plt.title("Predator Hunting Efficiency Over Generations")
plt.legend()
plt.grid(True)
plt.savefig("hunting_efficiency_trends.png")

#TODO: get data that will allow me to plot the correlation matrix over time
# generations_to_plot = np.arange(0, len(gen_list), 200)

# for generation in generations_to_plot:
#     subset = df[df["Generation"] == generation].drop(columns=["Generation"])
#     corr_matrix = subset.corr()

#     print(f"Correlation matrix for generation {generation}:\n{corr_matrix}")

#     plt.figure(figsize=(10, 8))
#     sns.heatmap(corr_matrix, annot=True, cmap="coolwarm", vmin=-1, vmax=1)
#     plt.title(f"Correlation Matrix for Generation {generation}")
#     plt.savefig(f"correlation_matrix_generation_{generation}.png")
#     plt.close()

# trait_pairs = [
#     ("Organism Size", "Predator Size"),
#     ("Organism Speed", "Predator Speed"),
#     ("Organism Energy", "Predator Energy"),
#     ("Organism Reproduction Threshold", "Predator Hunting Efficiency"),
# ]

# correlation_data = {pair: [] for pair in trait_pairs}
# correlation_data["Generation"] = []

# print("correlation_data", correlation_data)

# for generation in sorted(df["Generation"].unique()):
#     subset = df[df["Generation"] == generation]
#     correlation_data["Generation"].append(generation)
    
#     for trait1, trait2 in trait_pairs:
#         if trait1 in subset.columns and trait2 in subset.columns:
#             if len(subset[trait1]) > 1 and len(subset[trait2]) > 1:
#                 corr, _ = pearsonr(
#                     subset[trait1], subset[trait2]
#                 )
#                 correlation_data[(trait1, trait2)].append(corr)
#             else:
#                 correlation_data[(trait1, trait2)].append(None)

# corr_df = pd.DataFrame(correlation_data)
# plt.figure(figsize=(12, 8))
# for trait_pair in trait_pairs:
#     plt.plot(corr_df["Generation"], corr_df[trait_pair], label=f"{trait_pair[0]} vs {trait_pair[1]}")
# plt.xlabel("Generation")
# plt.ylabel("Correlation Coefficient")
# plt.title("Correlation Trend Over Generations")
# plt.legend()
# plt.grid(True)
# plt.savefig("correlation_trends_over_generations.png")
# plt.close()