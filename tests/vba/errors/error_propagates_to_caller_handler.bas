' INV: stmt.on-error
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/on-error-statement
' EXPECT: caught 13
Sub Inner()
    Error 13
End Sub
Public Sub Main()
    On Error GoTo H
    Inner
    Debug.Print "no"
    Exit Sub
H:
    Debug.Print "caught " & Err.Number
End Sub
