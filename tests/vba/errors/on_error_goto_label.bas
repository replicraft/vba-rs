' INV: stmt.on-error
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/on-error-statement
' EXPECT: handled 11
Public Sub Main()
    On Error GoTo Handler
    Debug.Print 1 / 0
    Debug.Print "not here"
    Exit Sub
Handler:
    Debug.Print "handled " & Err.Number
End Sub
