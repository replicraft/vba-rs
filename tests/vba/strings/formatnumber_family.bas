' INV: func.formatnumber, func.formatcurrency, func.formatpercent
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT: 1,234.57
' EXPECT: 0.50
' EXPECT: 25.6%
' EXPECT: $1,234.50
Public Sub Main()
    Debug.Print CStr(FormatNumber(1234.567, 2))
    Debug.Print CStr(FormatNumber(0.5, 2))
    Debug.Print CStr(FormatPercent(0.256, 1))
    Debug.Print CStr(FormatCurrency(1234.5, 2))
End Sub
