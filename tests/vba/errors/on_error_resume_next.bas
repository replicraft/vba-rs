' INV: stmt.on-error
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/on-error-statement
' EXPECT: after
' EXPECT:  11 
Public Sub Main()
    Dim x As Long
    On Error Resume Next
    x = 1 / 0
    Debug.Print "after"
    Debug.Print Err.Number
End Sub
