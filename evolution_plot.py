import pandas as pd
import matplotlib.pyplot as plt
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


world_data = pd.read_csv('world_data.csv')
print("Data preview:")
print(world_data.head())

# Pivot for food availability:
food_heatmap = world_data.pivot_table(index="y", columns="x", values="food_availability", aggfunc='mean')
organism_heatmap = world_data.pivot_table(index="y", columns="x", values="organism_count", aggfunc='sum')
predator_heatmap = world_data.pivot_table(index="y", columns="x", values="predator_count", aggfunc='sum')

sns.set_theme(font_scale=1.2)
plt.figure(figsize=(12, 10))

plt.subplot(3, 1, 1)
sns.heatmap(food_heatmap, cmap="YlGnBu", annot=True, fmt=".1f")
plt.title("Food Availability Heatmap")
plt.xlabel("X coordinate")
plt.ylabel("Y coordinate")

plt.subplot(3, 1, 2)
sns.heatmap(organism_heatmap, cmap="YlOrRd", annot=True, fmt="d")
plt.title("Organism Count Heatmap")
plt.xlabel("X coordinate")
plt.ylabel("Y coordinate")

plt.subplot(3, 1, 3)
sns.heatmap(predator_heatmap, cmap="Reds", annot=True, fmt="d")
plt.title("Predator Count Heatmap")
plt.xlabel("X coordinate")
plt.ylabel("Y coordinate")

plt.tight_layout()
plt.show()
plt.savefig('world_heatmaps.png')