## bank_account_parser
Проект для работы с финансовыми выписками который содержит:
1. bank_account_parser - библиотека для работы с финансовыми выписками в формате mt940 camt053 csv
2. comparer - утилита использующая библиотеку bank_account_parser для сравнения транзакций из двух файлов.


    Аргументы утилиты:
       --file1 <FILE1> // путь до первого файла
       --file1-format <FILE1_FORMAT> // формат первого файла
       --file2 <FILE2> // путь до второго файла
       --file2-format <FILE2_FORMAT> // формат второго файла
        Формат может быть одним из значений [camt053, mt940, csv]
    Пример вызова:
        --file1 "example_data/camt 053 treasurease" --file1-format camt053 --file2 "example_data/Пример выписки по счёту 1.csv" --file2-format csv

3. converter - утилита использующая библиотеку bank_account_parser для преобразования из формата mt940 в camt053 и наоборот


    Аргументы утилиты:
        --input <INPUT> // путь до файла
        --input-format <INPUT_FORMAT> // имходный формат файла
        Формат может быть одним из значений [camt053, mt940]
    Пример вызова:
        --input "example_data/camt 053 treasurease" --input-format camt053
