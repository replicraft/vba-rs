' INV: vbal.5.4.2.12, stmt.goto
' SOURCE: spec ms-vbal 5.4.2.12 GoTo may be written as two words
' EXPECT: reached
Public Sub Main()
    Go To Fin
    Debug.Print "skipped"
Fin:
    Debug.Print "reached"
End Sub
