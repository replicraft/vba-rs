' INV: obj.err
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/err-object
' EXPECT:  11 
' EXPECT: Division by zero
' EXPECT:  0 
' EXPECT: []
Public Sub Main()
    On Error Resume Next
    Error 11
    Debug.Print Err.Number
    Debug.Print Err.Description
    Err.Clear
    Debug.Print Err.Number
    Debug.Print "[" & Err.Description & "]"
End Sub
