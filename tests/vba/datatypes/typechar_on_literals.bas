' INV: typechar.amp, typechar.hash, typechar.bang, typechar.at, lit.integer, lit.float
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/data-type-summary
' EXPECT: LongDoubleSingleCurrencyInteger
Public Sub Main()
    Debug.Print TypeName(5&); TypeName(5#); TypeName(5!); TypeName(5@); TypeName(5)
End Sub
