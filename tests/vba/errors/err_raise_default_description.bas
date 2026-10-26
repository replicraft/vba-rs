' INV: obj.err
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/err-object
' EXPECT: Application-defined or object-defined error
Public Sub Main()
    On Error Resume Next
    Err.Raise 1000
    Debug.Print Err.Description
End Sub
