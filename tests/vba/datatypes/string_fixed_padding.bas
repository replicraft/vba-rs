' INV: type.string-fixed
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/data-type-summary
' EXPECT: [ab   ]5
' EXPECT: [abcde]
Public Sub Main()
    Dim s As String * 5
    s = "ab"
    Debug.Print "[" & s & "]" & Len(s)
    s = "abcdefgh"
    Debug.Print "[" & s & "]"
End Sub
