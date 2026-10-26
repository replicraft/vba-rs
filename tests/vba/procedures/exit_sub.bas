' INV: stmt.exit, stmt.sub
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/exit-statement
' EXPECT: a
Sub Quit()
    Debug.Print "a"
    Exit Sub
    Debug.Print "b"
End Sub
Public Sub Main()
    Quit
End Sub
