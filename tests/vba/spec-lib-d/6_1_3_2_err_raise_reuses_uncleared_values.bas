' INV: vbal.6.1.3.2.1.2
' SOURCE: spec ms-vbal 6.1.3.2.1.2 Raise without some arguments takes uncleared Err property values
' EXPECT: first=MySrc|MyDesc
' EXPECT: second=MySrc|MyDesc|5
Public Sub Main()
    On Error Resume Next
    Err.Raise 5, "MySrc", "MyDesc"
    Debug.Print "first=" & Err.Source & "|" & Err.Description
    Err.Raise 5
    Debug.Print "second=" & Err.Source & "|" & Err.Description & "|" & CStr(Err.Number)
End Sub
