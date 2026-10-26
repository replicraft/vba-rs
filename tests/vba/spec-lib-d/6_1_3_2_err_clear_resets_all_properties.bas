' INV: vbal.6.1.3.2.1.1
' SOURCE: spec ms-vbal 6.1.3.2.1.1 Err.Clear clears all property settings
' EXPECT: num=0
' EXPECT: desc=[]
' EXPECT: src=[]
' EXPECT: hc=0
' EXPECT: hf=[]
Public Sub Main()
    On Error Resume Next
    Err.Raise 5, "MySrc", "MyDesc", "f.hlp", 9
    Err.Clear
    Debug.Print "num=" & CStr(Err.Number)
    Debug.Print "desc=[" & Err.Description & "]"
    Debug.Print "src=[" & Err.Source & "]"
    Debug.Print "hc=" & CStr(Err.HelpContext)
    Debug.Print "hf=[" & Err.HelpFile & "]"
End Sub
