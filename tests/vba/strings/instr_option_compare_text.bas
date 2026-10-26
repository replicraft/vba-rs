' INV: func.instr, stmt.option-compare
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/instr-function
' EXPECT:  2 
Option Compare Text
Public Sub Main()
    Debug.Print InStr("ABC", "b")
End Sub
