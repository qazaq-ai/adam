fn main() {
    use adam_hybrid_llm::*;
    println!(
        "paraphrase: {:?}",
        propose_paraphrase("Қазақстанның астанасы қай қала?")
    );
    println!(
        "rescore_2: {:?}",
        rescore_n_best(&["алматы".into(), "астана".into()])
    );
    println!(
        "dialog_act:greeting: {:?}",
        classify_dialog_act("Сәлеметсіз бе!")
    );
    println!(
        "dialog_act:factual: {:?}",
        classify_dialog_act("Қазақстанның астанасы қай қала?")
    );
}
