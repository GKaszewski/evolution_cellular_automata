import json
import sys
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt
import matplotlib.animation as animation
import seaborn as sns

data = pd.read_csv('organism_data.csv')

plt.figure()
plt.plot(data["generation"], data["total_organisms"], label="Total Population")
#plt.plot(data["generation"], data["avg_energy"], label="Average Energy")
plt.xlabel("Generation")
plt.ylabel("Total Organisms")
plt.title("Organisms Population Over Time")
plt.legend()
plt.savefig('organisms_population_over_time.png')
# plt.show()

plt.figure()
plt.plot(data["generation"], data["avg_speed"], label="Average Speed")
plt.plot(data["generation"], data["avg_size"], label="Average Size")
plt.plot(data["generation"], data["avg_reproduction_threshold"], label="Average Reproduction Threshold")
#plt.plot(data["generation"], data["avg_energy"], label="Average Energy")
plt.xlabel("Generation")
plt.ylabel("Average Trait Value")
plt.title("Trait Evolution Over Time")
plt.legend()
plt.savefig('trait_evolution_over_time.png')
# plt.show()

predators_data = pd.read_csv('predator_data.csv')

plt.figure()
plt.plot(predators_data["generation"], predators_data["total_predators"], label="Total Predators")
#plt.plot(predators_data["generation"], predators_data["avg_energy"], label="Average Energy")
plt.xlabel("Generation")
plt.ylabel("Total Predators")
plt.title("Predator Population Over Time")
plt.legend()
plt.savefig('predator_population_over_time.png')

plt.figure()
plt.plot(predators_data["generation"], predators_data["avg_speed"], label="Average Speed")
plt.plot(predators_data["generation"], predators_data["avg_size"], label="Average Size")
#plt.plot(predators_data["generation"], predators_data["avg_reproduction_threshold"], label="Average Reproduction Threshold")
#plt.plot(predators_data["generation"], predators_data["avg_energy"], label="Average Energy")
plt.xlabel("Generation")
plt.ylabel("Average Trait Value")
plt.title("Predator Trait Evolution Over Time")
plt.legend()
plt.savefig('predator_trait_evolution_over_time.png')


snapshots = []
with open('world_data.jsonl', 'r') as f:
    for line in f:
        if not line.strip():
            continue
        snapshots.append(json.loads(line.strip()))


width = snapshots[0]["config"]["width"]
height = snapshots[0]["config"]["height"]

generations = [data["generation"] for data in snapshots]
organisms_count = [len(data["organisms"]) for data in snapshots]
predators_count = [len(data["predators"]) for data in snapshots]

plt.figure(figsize=(10, 5))
plt.plot(generations, organisms_count, label="Organisms", color="lime", linewidth=2)
plt.plot(generations, predators_count, label="Predators", color="red", linewidth=2)
plt.xlabel("Generation")
plt.ylabel("Population")
plt.title("Population Over Time")
plt.legend()
plt.grid(True)
plt.savefig('population_trends.png')

heatmap_grid = np.zeros((height, width))
for data in snapshots:
    for org in data["organisms"]:
        x,y = org["position"]["x"], org["position"]["y"]
        heatmap_grid[y][x] += 1
    for pred in data["predators"]:
        x,y = pred["position"]["x"], pred["position"]["y"]
        heatmap_grid[y][x] += 1

plt.figure(figsize=(8, 6))
sns.heatmap(heatmap_grid, cmap="hot", square=True)
plt.title("Population Heatmap")
plt.xlabel("X")
plt.ylabel("Y")
plt.savefig('population_heatmap.png')

organism_energies = [org["organism"]["energy"] for data in snapshots for org in data["organisms"]]
predator_energies = [pred["predator"]["energy"] for data in snapshots for pred in data["predators"]]

plt.figure(figsize=(10, 5))
sns.histplot(organism_energies, bins=30, color="lime", alpha=0.7, label="Organisms", kde=True)
sns.histplot(predator_energies, bins=30, color="red", alpha=0.7, label="Predators", kde=True)
plt.xlabel("Energy Levels")
plt.ylabel("Frequency")
plt.title("Energy Distribution of Organisms and Predators")
plt.legend()
plt.grid(True)
plt.savefig("energy_distribution.png")

biome_counts = {"Forest": [], "Desert": [], "Water": [], "Grassland": []}
for data in snapshots:
    biome_tally = {"Forest": 0, "Desert": 0, "Water": 0, "Grassland": 0}
    for org in data["organisms"]:
        max_biome = max(org["organism"]["biome_tolerance"], key=org["organism"]["biome_tolerance"].get)
        biome_tally[max_biome] += 1
    for biome in biome_counts:
        biome_counts[biome].append(biome_tally[biome])

df_biomes = pd.DataFrame(biome_counts, index=generations)

df_biomes.plot(kind="area", stacked=True, figsize=(10, 6), colormap="coolwarm", alpha=0.7)
plt.xlabel("Generation")
plt.ylabel("Organism Count")
plt.title("Biome Preference Trends Over Generations")
plt.legend(title="Biome")
plt.grid(True)
plt.savefig("biome_trends.png")

average_food_per_generation = []
for data in snapshots:
    total_food = sum(tile["food_availabilty"] for row in data["world"]["grid"] for tile in row)
    average_food_per_generation.append(total_food / (width * height)) # normalize by world size

plt.figure(figsize=(10, 5))
plt.plot([data["generation"] for data in snapshots], average_food_per_generation, label="Avg Food Availability", color="orange", linewidth=2)
plt.xlabel("Generation")
plt.ylabel("Average Food Availability")
plt.title("Food Availability Trends Over Generations")
plt.legend()
plt.grid(True)
plt.savefig("food_trends.png")

last_snapshot = snapshots[-1]  # Get the last generation
food_grid = np.array([[tile["food_availabilty"] for tile in row] for row in last_snapshot["world"]["grid"]])

plt.figure(figsize=(8, 6))
sns.heatmap(food_grid, cmap="YlGnBu", square=True)
plt.title(f"Food Availability Heatmap (Generation {last_snapshot['generation']})")
plt.xlabel("X Position")
plt.ylabel("Y Position")
plt.savefig("food_heatmap.png")

organism_avg_size = []
organism_avg_speed = []
organism_avg_energy = []
predator_avg_size = []
predator_avg_speed = []
predator_avg_energy = []
organism_avg_reproduction_threshold = []
predator_avg_reproduction_threshold = []
predator_avg_hunting_efficiency = []
predator_avg_satiation_threshold = []

for data in snapshots:
    if data["organisms"]:
        organism_avg_size.append(np.mean([o["organism"]["size"] for o in data["organisms"]]))
        organism_avg_speed.append(np.mean([o["organism"]["speed"] for o in data["organisms"]]))
        organism_avg_energy.append(np.mean([o["organism"]["energy"] for o in data["organisms"]]))
        organism_avg_reproduction_threshold.append(np.mean([o["organism"]["reproduction_threshold"] for o in data["organisms"]]))
    else:
        organism_avg_size.append(0)
        organism_avg_speed.append(0)
        organism_avg_energy.append(0)
        organism_avg_reproduction_threshold.append(0)

    if data["predators"]:
        predator_avg_size.append(np.mean([p["predator"]["size"] for p in data["predators"]]))
        predator_avg_speed.append(np.mean([p["predator"]["speed"] for p in data["predators"]]))
        predator_avg_energy.append(np.mean([p["predator"]["energy"] for p in data["predators"]]))
        predator_avg_reproduction_threshold.append(np.mean([p["predator"]["reproduction_threshold"] for p in data["predators"]]))
        predator_avg_hunting_efficiency.append(np.mean([p["predator"]["hunting_efficiency"] for p in data["predators"]]))
        predator_avg_satiation_threshold.append(np.mean([p["predator"]["satiation_threshold"] for p in data["predators"]]))
    else:
        predator_avg_size.append(0)
        predator_avg_speed.append(0)
        predator_avg_energy.append(0)
        predator_avg_reproduction_threshold.append(0)
        predator_avg_hunting_efficiency.append(0)
        predator_avg_satiation_threshold.append(0)

fig, axes = plt.subplots(3, 1, figsize=(10, 12))
axes[0].plot(generations, organism_avg_size, label="Organisms - Avg Size", color="lime", linewidth=2)
axes[0].plot(generations, predator_avg_size, label="Predators - Avg Size", color="red", linewidth=2)
axes[0].set_ylabel("Size")
axes[0].set_title("Evolution of Size Over Generations")
axes[0].legend()
axes[0].grid(True)

axes[1].plot(generations, organism_avg_speed, label="Organisms - Avg Speed", color="blue", linewidth=2)
axes[1].plot(generations, predator_avg_speed, label="Predators - Avg Speed", color="orange", linewidth=2)
axes[1].set_ylabel("Speed")
axes[1].set_title("Evolution of Speed Over Generations")
axes[1].legend()
axes[1].grid(True)

axes[2].plot(generations, organism_avg_energy, label="Organisms - Avg Energy", color="purple", linewidth=2)
axes[2].plot(generations, predator_avg_energy, label="Predators - Avg Energy", color="brown", linewidth=2)
axes[2].set_xlabel("Generation")
axes[2].set_ylabel("Energy")
axes[2].set_title("Evolution of Energy Over Generations")
axes[2].legend()
axes[2].grid(True)

plt.tight_layout()
plt.savefig("traits_evolution.png")

df = pd.DataFrame({
    "Generation": generations,
    "Organism Size": organism_avg_size,
    "Predator Size": predator_avg_size,
    "Organism Speed": organism_avg_speed,
    "Predator Speed": predator_avg_speed,
    "Organism Energy": organism_avg_energy,
    "Predator Energy": predator_avg_energy,
})

plt.figure(figsize=(8, 6))
sns.scatterplot(x="Organism Speed", y="Predator Speed", hue="Generation", size="Generation", sizes=(20, 200), data=df, palette="coolwarm")
plt.xlabel("Organism Speed")
plt.ylabel("Predator Speed")
plt.title("Correlation Between Organism and Predator Speed")
plt.legend(title="Generation", bbox_to_anchor=(1, 1))
plt.savefig("correlation_speed.png")

plt.figure(figsize=(8, 6))
sns.scatterplot(x="Organism Size", y="Predator Size", hue="Generation", size="Generation", sizes=(20, 200), data=df, palette="coolwarm")
plt.xlabel("Organism Size")
plt.ylabel("Predator Size")
plt.title("Correlation Between Organism and Predator Size")
plt.legend(title="Generation", bbox_to_anchor=(1, 1))
plt.savefig("correlation_size.png")

plt.figure(figsize=(10, 5))
plt.plot(generations, organism_avg_reproduction_threshold, label="Organisms - Reproduction Threshold", color="lime", linewidth=2)
plt.plot(generations, predator_avg_reproduction_threshold, label="Predators - Reproduction Threshold", color="red", linewidth=2)
plt.xlabel("Generation")
plt.ylabel("Reproduction Threshold")
plt.title("Reproduction Threshold Evolution Over Generations")
plt.legend()
plt.grid(True)
plt.savefig("reproduction_threshold_trends.png")

plt.figure(figsize=(10, 5))
plt.plot(generations, predator_avg_hunting_efficiency, label="Predators - Hunting Efficiency", color="blue", linewidth=2)
plt.xlabel("Generation")
plt.ylabel("Hunting Efficiency")
plt.title("Predator Hunting Efficiency Over Generations")
plt.legend()
plt.grid(True)
plt.savefig("hunting_efficiency_trends.png")

df = pd.DataFrame({
    "Generation": generations,
    "Organism Reproduction Threshold": organism_avg_reproduction_threshold,
    "Predator Hunting Efficiency": predator_avg_hunting_efficiency
})

plt.figure(figsize=(8, 6))
sns.scatterplot(x="Organism Reproduction Threshold", y="Predator Hunting Efficiency",
                hue="Generation", size="Generation", sizes=(20, 200), data=df, palette="coolwarm")
plt.xlabel("Organism Reproduction Threshold")
plt.ylabel("Predator Hunting Efficiency")
plt.title("Correlation Between Reproduction and Hunting Efficiency")
plt.legend(title="Generation", bbox_to_anchor=(1, 1))
plt.savefig("correlation_reproduction_hunting.png")

BIOME_COLORS = {
    "Forest": (0, 0.5, 0),
    "Desert": (0.96, 0.8, 0.6),
    "Water": (0, 0, 1),
    "Grassland": (0.48, 0.99, 0),
}

fig, ax = plt.subplots(figsize=(10, 10))
export_animation = sys.argv[1] == "export" if len(sys.argv) > 1 else False

def update(frame):
    ax.clear()
    data = snapshots[frame]
    generation = data["generation"]

    biome_grid = np.array([
        [BIOME_COLORS[tile["biome"]] for tile in row]
        for row in data["world"]["grid"]
    ])
    ax.imshow(biome_grid, aspect="equal")

    organism_x = [o["position"]["x"] for o in data["organisms"]]
    organism_y = [o["position"]["y"] for o in data["organisms"]]
    organism_sizes = [o["organism"]["size"] * 30 for o in data["organisms"]]
    ax.scatter(organism_x, organism_y, s=organism_sizes, c="grey", label="Organisms", alpha=0.6, edgecolors="black")

    predator_x = [p["position"]["x"] for p in data["predators"]]
    predator_y = [p["position"]["y"] for p in data["predators"]]
    predator_sizes = [p["predator"]["size"] * 50 for p in data["predators"]]
    ax.scatter(predator_x, predator_y, s=predator_sizes, c="red", label="Predators", alpha=0.8, edgecolors="black")

    ax.set_title(f"Generation {generation}")
    ax.set_xticks([])
    ax.set_yticks([])
    ax.legend()

    print(f"Processed frame {frame}/{len(snapshots)} ({frame / len(snapshots) * 100:.2f}%)")

if export_animation:
    ani = animation.FuncAnimation(fig, update, frames=len(snapshots), interval=100, repeat=False)
    ani.save('evolution.mp4', writer='ffmpeg', fps=24)