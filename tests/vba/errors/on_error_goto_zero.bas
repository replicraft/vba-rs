' INV: stmt.on-error
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/on-error-statement
' EXPECT: x
' ERROR: 11
Public Sub Main()
    On Error Resume Next
    Error 5
    On Error GoTo 0
    Debug.Print "x"
    Error 11
    Debug.Print "never"
End Sub
