import pandas as pd
import matplotlib.pyplot as plt

data = pd.read_csv('organism_data.csv')

plt.figure()
plt.plot(data["generation"], data["total_organisms"], label="Total Population")
plt.xlabel("Generation")
plt.ylabel("Total Organisms")
plt.title("Population Over Time")
plt.legend()
plt.savefig('population_over_time.png')
# plt.show()

plt.figure()
plt.plot(data["generation"], data["avg_speed"], label="Average Speed")
plt.plot(data["generation"], data["avg_size"], label="Average Size")
plt.plot(data["generation"], data["avg_reproduction_threshold"], label="Average Reproduction Threshold")
plt.xlabel("Generation")
plt.ylabel("Average Trait Value")
plt.title("Trait Evolution Over Time")
plt.legend()
plt.savefig('trait_evolution_over_time.png')
# plt.show()