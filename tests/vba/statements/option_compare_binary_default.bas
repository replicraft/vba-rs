' INV: stmt.option-compare
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/option-compare-statement
' EXPECT:  1 
Public Sub Main()
    Debug.Print StrComp("a", "A")
End Sub
