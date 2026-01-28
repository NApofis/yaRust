use std::collections::HashSet;

// Вспомогательный комбинатор итераторов
enum EitherIt<It1,It2> {
    It1(It1),
    It2(It2),
}
impl<It1,It2,T> Iterator for EitherIt<It1,It2>
where It1: Iterator<Item=T>,
      It2: Iterator<Item=T>
{
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::It1(it1) => it1.next(),
            Self::It2(it2) => it2.next(),
        }
    }
}

// Исправляет строки: заглавные после точек, термины (сравниваем регистро-независимо) - по шаблону
fn fix(bytes_to_fix: &str, set_of_labels: &HashSet<String>, id: usize) -> String {
    fn to_lower_iter(word: &str) -> impl Iterator<Item=char> {
        // Rust старается учитывать, что заглавная и строчная
        // unicode-буква может занимать разное число байт в utf-8
        word.chars().map(char::to_lowercase).flatten()
    }

    let mut shall_start_from_capital = true;

    let mut percent_idx = 0;
    fn get_bytes_diff (string: &str, substring: &str) -> usize {
        substring.as_ptr() as usize - string.as_ptr() as usize
    }

    bytes_to_fix
        .split_whitespace()
        .map(
            |orig_word| {
                if percent_idx < (get_bytes_diff(bytes_to_fix, orig_word) * 100) / bytes_to_fix.len() {
                    println!("[{}]: {}%", id, percent_idx);
                    percent_idx += 10;
                }

                let mut remaining = orig_word.chars();
                let first_char = remaining.next().unwrap();
                let first_char
                    = if shall_start_from_capital {
                    EitherIt::It1(first_char.to_uppercase())
                }
                else {
                    EitherIt::It2(first_char.to_lowercase())
                };
                shall_start_from_capital
                    = *orig_word.as_bytes().last().unwrap() == ('.' as u8);
                if let Some(label)
                    = set_of_labels.iter()
                    .find( |label|
                        to_lower_iter(&label)
                            .eq(to_lower_iter(orig_word)) )
                {
                    EitherIt::It1(label.chars().chain([' ']))
                }
                else {
                    EitherIt::It2(first_char.chain(to_lower_iter(remaining.as_str()).chain([' '])))
                }
            }
        )
        .flatten()
        .collect()
}

// Делит строку на более-менее равные части
fn split_n_impl(bytes: &[u8], from_idx: usize, to_idx: usize, parts_count: usize) -> Vec<&str> {
    assert!(parts_count != 0);
    if parts_count == 1 {
        vec![str::from_utf8(&bytes[from_idx..to_idx]).unwrap()]
    }
    else {
        let mut idx = from_idx + (to_idx - from_idx) / 2;
        while idx != 1 && &bytes[idx-1..=idx] != ". ".as_bytes() {
            idx -= 1;
        }
        if idx == 0 {
            vec![str::from_utf8(&bytes[from_idx..to_idx]).unwrap()]
        }
        else if parts_count == 2 {
            vec![
                str::from_utf8(&bytes[from_idx..idx]).unwrap(),
                str::from_utf8(&bytes[idx+1..to_idx]).unwrap()
            ]
        }
        else {
            [ split_n_impl(bytes, from_idx, idx, parts_count / 2),
                split_n_impl(bytes, idx + 1, to_idx, (parts_count + 1) / 2) ].concat()
        }
    }
}

fn split_n(s: &str, parts_count: usize) -> Vec<&str> {
    split_n_impl(s.as_bytes(), 0, s.len(), parts_count)
}

fn run_parallel(string: String, set_of_labels: &HashSet<String>) -> String {
    std::thread::scope(
        |scope| {
            let threads_count = 4;
            let parts = split_n(&string, threads_count);

            let mut result = String::new();
            let threads  = parts.into_iter().enumerate().map(|part| {
                return scope.spawn(move || fix(part.1, set_of_labels, part.0))
            }).collect::<Vec<_>>();

            for thread in threads {
                let done_part = thread.join().unwrap();
                result.extend(done_part.chars());
            }

            result.truncate(result.len() - 1); // убираем завершающий пробел
            result
        })
}


pub fn text_updater() {
    // Для простоты кода мы объединили многословные термины подчёркиванием,
    // а термины, кончающиеся знаками препинания, выделили в отдельные термины
    let orig = "Rust is used in several backend software projects of large web \
                services. OpenDNS, a DNS resolution service owned by Cisco, \
                uses Rust internally. Amazon_Web_Services uses Rust in \
                \"performance-sensitive components\" of its several services. \
                In 2019, AWS open-sourced Firecracker, a virtualization \
                solution primarily written in Rust. Microsoft Azure IoT Edge, \
                a platform used to run Azure services on IoT devices, has \
                components implemented in Rust. Microsoft also uses Rust to run \
                containerized modules with WebAssembly and Kubernetes. \
                Cloudflare, a company providing content delivery network \
                services, used Rust to build a new web proxy named Pingora for \
                increased performance and efficiency. The npm package manager \
                used Rust for its production authentication service in 2019.";

    let mix  = "RUst Is useD In seVeRal bacKend SOfTWArE ProjECTS OF LarGe wEb \
                SERVICEs. oPENdNS, A DNS REsOlution sErvIce owneD by cIScO, \
                uSES ruST inteRNaLLY. AMAzon_Web_SERvICEs uSEs rUST iN \
                \"peRFOrmAnce-SEnSitivE COmPoneNtS\" OF iTs SeVeRal SeRViceS. \
                In 2019, AWS oPEN-sOurCeD fIREcRaCKeR, A vIrTUAliZaTIOn \
                solUtIon PrIMARiLY WrITteN iN Rust. MIcROSoFT AZuRE Iot edge, \
                a platfOrm used tO rUN AzURe SErviCES on iOT DEvices, haS \
                COMponEnts iMPLEmeNtED in RusT. mIcrOsOFt ALSO usEs RuSt to RUn \
                CONtAinerIZED ModUlES WIth wEbasSEMBly anD kuBERNETEs. \
                CloUdFlare, a cOMPAny proVIDiNg cONTeNT DEliVEry neTwOrK \
                SErViCes, used ruSt TO bUILD A NeW weB pRoxY NaMED PIngora For \
                iNCreASEd pERFOrMANCE And EFfiCieNcy. the NPM PAckAgE mAnAgEr \
                uSed RuST FOR ITS PrODuctIOn AUTheNTIcaTiON sErvIce IN 2019.";

    let dict = [ "Rust", "Rust,", "Rust.", "OpenDNS,", "DNS", "Cisco,",
        "Amazon_Web_Services", "AWS", "Firecracker,", "Microsoft",
        "Azure", "IoT", "Edge,", "WebAssembly", "Kubernetes.",
        "Cloudflare", "Pingora" ];

    let result: String;

    let handler = std::thread::spawn(move || run_parallel(
        mix.to_string(),
        &dict.iter().map(|x| x.to_string()).collect::<HashSet<String>>()));

    if let Ok(res) = handler.join() {
        result = res;
    } else {
        result = "Error: Failed to join thread".to_string();
    }

    println!("{result}");
    assert_eq!(result, orig);
}