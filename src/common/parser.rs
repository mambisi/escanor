pub fn parse_raw_cmd(cmd: &[u8]) -> Vec<String> {


    let mut tokens: Vec<String> = vec![];

    //let cmd = cmd.trim();

    let mut block_seq = String::new();
    let mut in_string = false;
    let mut next_char = '\0';
    let mut prev_char = '\0';
    let text_qualifier = '`';
    let text_delimiter = ' ';

    for (i, b) in cmd.into_iter().enumerate() {
        let current_char = *b as char;

        let block = &mut block_seq;

        if i > 0 {
            prev_char = cmd[i - 1] as char;
        } else {
            prev_char = '\0';
        }

        if i + 1 < cmd.len() {
            next_char = cmd[i + 1] as char;
        } else {
            next_char = '\0';
        }


        if current_char == text_qualifier && (prev_char == '\0' || prev_char == text_delimiter) && !in_string {
            in_string = true;
            continue;
        }

        if current_char == text_qualifier && (next_char == '\0' || next_char == text_delimiter) && in_string {
            in_string = false;
            continue;
        }

        // ignoring whitespace follow by white space which is not in a string
        if current_char == ' ' && next_char == ' ' && !in_string {
            continue;
        }


        if current_char == text_delimiter && !in_string {
            let token = block.clone();
            tokens.push(token);
            block_seq.clear();
            continue;
        }
        block_seq.push(current_char);
    }
    //tokens.push(block_seq.trim_end().trim_end_matches(&['`'] as &[_]).to_owned());
    tokens.push(block_seq);
    return tokens;
}