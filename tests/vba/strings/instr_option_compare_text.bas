' INV: func.instr, stmt.option-compare
' SOURCE: spec ms-vbal 5.2.1.1 Option Compare Text and 6.1.2.11.1.14 InStr
' EXPECT:  2 
Option Compare Text
Public Sub Main()
    Debug.Print InStr("ABC", "b")
End Sub
