## Currencies & Commodities Unit
Currencies treated as the same as Commodities and called unit.
The operating unit will need to be declared with `unit` directive.

```
; example:
unit JPY
```

The consecutive units for more currencies / commodities will need to be
defined as `price` statement.

```
2024/10/13 price USD 100 JPY

; on 2024/10/13 or later, unit `USD` can be used with conversion rate at
; 100 JPY / USD
; If `Assets:Bank:A' operate in JPY, transaction with USD unit can
; automatically convert the correct amount from JPY
2024/10/13 * "Bill payable"
  Assets:Bank:A                    ;amount elided == 14_200 JPY
  Expenses:Foreign:Bill    142 USD
```

Price can be defined several times and the latest one will get reflected for the next transaction.
