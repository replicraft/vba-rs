' INV: stmt.exit, stmt.sub
' SOURCE: spec ms-vbal 5.4.2.17 Exit Sub statement
' EXPECT: a
Sub Quit()
    Debug.Print "a"
    Exit Sub
    Debug.Print "b"
End Sub
Public Sub Main()
    Quit
End Sub
