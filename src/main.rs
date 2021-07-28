use err_derive::Error;
use petgraph::graph::NodeIndex;
use petgraph::{graph::DiGraph, visit::IntoNodeReferences};
use std::error::Error;
use std::{collections::HashMap, fmt::Display};

#[derive(Debug, Error)]
enum RespiError {
    #[error(display = "csv data is invalid or could not be read")]
    CsvError(),
}

type RespiGraph = DiGraph<RespiNode, usize>;

struct Respi {
    graph: DiGraph<RespiNode, usize>,
}

impl Respi {
    fn init(item_csv_path: String) -> Result<Respi, Box<dyn Error>> {
        let mut graph = RespiGraph::new();
        let (new_items, new_syntheses, new_morphs) = Respi::parse_csv(item_csv_path)?;

        let mut item_indices = HashMap::new();

        for new_item in &new_items {
            let i = graph.add_node(RespiNode::Item {
                name: new_item.name.clone(),
                fire: new_item.fire,
                ice: new_item.ice,
                light: new_item.light,
                wind: new_item.wind,
                category1: new_item.category1.clone(),
                category2: new_item.category2.clone(),
                category3: new_item.category3.clone(),
                category4: new_item.category4.clone(),
                item_number: new_item.item_number,
            });
            item_indices.insert(&new_item.name, i);
        }

        for new_synthesis in &new_syntheses {
            let synth_index = graph.add_node(RespiNode::Synthesis {
                chapter: new_synthesis.chapter.clone(),
                synthesis_type: new_synthesis.synthesis_type.clone(),
                add_category1: new_synthesis.add_category1.clone(),
                add_category2: new_synthesis.add_category2.clone(),
                extra_synth_quantity: new_synthesis.extra_synth_quantity,
                effect_spread: new_synthesis.effect_spread,
            });

            if let Some(item_index) = item_indices.get(&new_synthesis.name) {
                graph.add_edge(synth_index, *item_index, 0);

                for ingredient in new_synthesis.ingredients() {
                    let ingredients: Vec<_> = graph
                        .node_references()
                        .filter(|(_, n)| match n {
                            RespiNode::Item {
                                name,
                                category1,
                                category2,
                                category3,
                                category4,
                                ..
                            } => [
                                Some(name),
                                category1.as_ref(),
                                category2.as_ref(),
                                category3.as_ref(),
                                category4.as_ref(),
                            ]
                            .iter()
                            .any(|c| c == &Some(&ingredient)),
                            _ => false,
                        })
                        .map(|(i, _)| i)
                        .collect();

                    for ingredient_index in ingredients {
                        graph.add_edge(ingredient_index, synth_index, 0);
                    }
                }
            }
        }

        for new_morph in &new_morphs {
            let result_index = item_indices[&new_morph.name];
            let required_item_index = item_indices[&new_morph.from_requiring];
            let recipe_index = graph
                .node_references()
                .find(|(_, n)| match n {
                    RespiNode::Item { name, .. } => name == &new_morph.from_recipe,
                    _ => false,
                })
                .map(|(i, _)| graph.neighbors_directed(i, petgraph::Direction::Incoming))
                .expect(
                    &format!(
                        "a recipe {} exsists as a base synthesis for this morph",
                        &new_morph.from_recipe
                    )[..],
                )
                .find(|i| matches!(&graph[*i], RespiNode::Synthesis { .. }))
                .expect("there");

            let morph_index = graph.add_node(RespiNode::Morph);
            graph.add_edge(recipe_index, morph_index, 0);
            graph.add_edge(required_item_index, morph_index, 0);
            graph.add_edge(morph_index, result_index, 0);
        }

        Ok(Respi { graph })
    }

    fn parse_csv(
        item_csv_path: String,
    ) -> Result<(Vec<NewItem>, Vec<NewSynthesis>, Vec<NewMorph>), Box<dyn Error>> {
        let mut reader = csv::Reader::from_path(item_csv_path)?;
        let mut new_items = Vec::new();
        let mut new_syntheses = Vec::new();
        let mut new_morphs = Vec::new();

        for record in reader.records() {
            let record = record?;

            if record.len() != 25 {
                return Err(Box::new(RespiError::CsvError()));
            } else {
                fn empty_or_some(text: &str) -> Option<String> {
                    if text.is_empty() {
                        None
                    } else {
                        Some(text.to_owned())
                    }
                }

                let name = record[0].to_string();
                let fire = !record[1].is_empty();
                let ice = !record[2].is_empty();
                let light = !record[3].is_empty();
                let wind = !record[4].is_empty();
                let category1 = empty_or_some(&record[5]);
                let category2 = empty_or_some(&record[6]);
                let category3 = empty_or_some(&record[7]);
                let category4 = empty_or_some(&record[8]);
                let item_number = if !record[9].is_empty() {
                    ItemNumber::MaterialNumber(record[9].parse::<u8>()?)
                } else if !record[10].is_empty() {
                    ItemNumber::RecipeNumber(record[10].parse::<u8>()?)
                } else {
                    ItemNumber::None
                };

                if let ItemNumber::RecipeNumber(_) = item_number {
                    let chapter = record[11].to_string();
                    let synthesis_type = record[12].to_string();
                    let ingredient1 = empty_or_some(&record[13]);
                    let ingredient2 = empty_or_some(&record[14]);
                    let ingredient3 = empty_or_some(&record[15]);
                    let ingredient4 = empty_or_some(&record[16]);

                    let add_category1 = empty_or_some(&record[17]);
                    let add_category2 = empty_or_some(&record[18]);
                    let from_recipe1 = empty_or_some(&record[19]);
                    let from_requiring1 = empty_or_some(&record[20]);
                    let from_recipe2 = empty_or_some(&record[21]);
                    let from_requiring2 = empty_or_some(&record[22]);

                    let extra_synth_quantity = if record[23].is_empty() {
                        None
                    } else {
                        Some(record[23].parse::<u8>()?)
                    };
                    let effect_spread = if record[24].is_empty() {
                        None
                    } else {
                        Some(record[24].parse::<u8>()?)
                    };

                    new_syntheses.push(NewSynthesis {
                        name: name.clone(),
                        chapter: chapter.clone(),
                        synthesis_type,
                        ingredient1: ingredient1.clone(),
                        ingredient2,
                        ingredient3,
                        ingredient4,
                        add_category1,
                        add_category2,
                        extra_synth_quantity,
                        effect_spread,
                    });

                    if let (Some(from_recipe), Some(from_requiring)) =
                        (from_recipe1, from_requiring1)
                    {
                        new_morphs.push(NewMorph {
                            name: name.clone(),
                            chapter: chapter.clone(),
                            from_recipe,
                            from_requiring,
                        });
                    }

                    if let (Some(from_recipe), Some(from_requiring)) =
                        (from_recipe2, from_requiring2)
                    {
                        new_morphs.push(NewMorph {
                            name: name.clone(),
                            chapter: chapter.clone(),
                            from_recipe,
                            from_requiring,
                        });
                    }
                };

                new_items.push(NewItem {
                    name,
                    fire,
                    ice,
                    light,
                    wind,
                    category1,
                    category2,
                    category3,
                    category4,
                    item_number,
                });
            }
        }

        Ok((new_items, new_syntheses, new_morphs))
    }

    fn find_item(&self, item_name: &str) -> Option<NodeIndex> {
        self.graph.node_indices().find(|i| match &self.graph[*i] {
            RespiNode::Item { name, .. } => name == item_name,
            _ => false,
        })
    }

    #[allow(unreachable_code)]
    fn run(self) -> Result<(), Box<dyn Error>> {
        use std::io::{stdin, stdout, Write};

        fn get_input(target: &mut String, prompt: &str) {
            print!("{} ", prompt);
            stdout().flush().unwrap();
            stdin().read_line(target).unwrap();
            target.pop();
        }

        loop {
            let start_index = loop {
                let mut start_name = String::new();
                get_input(&mut start_name, "start:");
                if let Some(node_index) = &self.find_item(&start_name) {
                    break node_index.clone();
                }
            };

            let goal_index = loop {
                let mut goal_name = String::new();
                get_input(&mut goal_name, "goal:");
                if let Some(node_index) = &self.find_item(&goal_name) {
                    break node_index.clone();
                }
            };

            print!("shortest path: ");
            if let Some((_, path)) = petgraph::algo::astar(
                &self.graph,
                start_index,
                |finish| finish == goal_index,
                |_| 1,
                |_| 0,
            ) {
                for ni in path {
                    print!("{}", &self.graph[ni]);
                    if let RespiNode::Item { name, .. } = &self.graph[ni] {
                        if let RespiNode::Item {
                            name: goal_name, ..
                        } = &self.graph[goal_index]
                        {
                            if name != goal_name {
                                print!(" -> ");
                            }
                        }
                    } else {
                        print!(" -> ");
                    }
                }
            }
            println!("\n");
        }

        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
enum ItemNumber {
    MaterialNumber(u8),
    RecipeNumber(u8),
    None,
}

#[derive(Clone, Debug)]
enum RespiNode {
    Synthesis {
        chapter: String,
        synthesis_type: String,
        add_category1: Option<String>,
        add_category2: Option<String>,
        extra_synth_quantity: Option<u8>,
        effect_spread: Option<u8>,
    },
    Morph,
    Item {
        name: String,
        fire: bool,
        ice: bool,
        light: bool,
        wind: bool,
        category1: Option<String>,
        category2: Option<String>,
        category3: Option<String>,
        category4: Option<String>,
        item_number: ItemNumber,
    },
}

impl Display for RespiNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Synthesis { .. } => {
                write!(f, "Synthesis")
            }
            Self::Morph => {
                write!(f, "Morph")
            }
            Self::Item { name, .. } => {
                write!(f, "{}", name)
            }
        }
    }
}

#[derive(Debug)]
struct NewSynthesis {
    name: String,
    chapter: String,
    synthesis_type: String,
    ingredient1: Option<String>,
    ingredient2: Option<String>,
    ingredient3: Option<String>,
    ingredient4: Option<String>,
    add_category1: Option<String>,
    add_category2: Option<String>,
    extra_synth_quantity: Option<u8>,
    effect_spread: Option<u8>,
}

impl NewSynthesis {
    fn ingredients(&self) -> Vec<String> {
        [
            &self.ingredient1,
            &self.ingredient2,
            &self.ingredient3,
            &self.ingredient4,
        ]
        .iter()
        .filter(|i| i.is_some())
        .map(|i| i.as_ref().unwrap().clone())
        .collect()
    }
}

#[derive(Debug)]
struct NewMorph {
    name: String,
    chapter: String,
    from_recipe: String,
    from_requiring: String,
}

#[derive(Debug, Clone)]
struct NewItem {
    name: String,
    fire: bool,
    ice: bool,
    light: bool,
    wind: bool,
    category1: Option<String>,
    category2: Option<String>,
    category3: Option<String>,
    category4: Option<String>,
    item_number: ItemNumber,
}

fn print_help() {
    println!("Usage:\n  respi [OPTION]\n\nOptions:\n  -i, --items <file>\t\tcsv file containing all items");
}

fn main() {
    let mut args = std::env::args();
    let _program_name = args.next();
    let mut item_csv_path = String::new();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "respi" => {}
            "-i" | "--items" => {
                if let Some(filepath) = args.next() {
                    item_csv_path = filepath
                } else {
                    print_help()
                }
            }
            _ => print_help(),
        }
    }

    if let Ok(respi) = Respi::init(item_csv_path) {
        match respi.run() {
            Err(error) => {
                println!("{}", error);
            }
            _ => {}
        }
    }
}
