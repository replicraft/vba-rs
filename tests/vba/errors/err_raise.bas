' INV: obj.err
' SOURCE: spec ms-vbal 6.1.3.2.1.2 Err.Raise
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
