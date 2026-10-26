' INV: obj.err
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/err-object
' EXPECT:  1234 
' EXPECT: MySource
' EXPECT: custom message
Public Sub Main()
    On Error Resume Next
    Err.Raise 1234, "MySource", "custom message"
    Debug.Print Err.Number
    Debug.Print Err.Source
    Debug.Print Err.Description
End Sub
