' INV: vbal.6.1.3.2.2.5
' SOURCE: spec ms-vbal 6.1.3.2.2.5 Err.Number is the default property of Err; Number is Long and settable
' EXPECT: def=11
' EXPECT: num=11
' EXPECT: t=Long
' EXPECT: set=77
Public Sub Main()
    Dim x As Variant
    On Error Resume Next
    Err.Raise 11
    x = Err
    Debug.Print "def=" & CStr(x)
    Debug.Print "num=" & CStr(Err.Number)
    Debug.Print "t=" & TypeName(Err.Number)
    Err.Number = 77
    Debug.Print "set=" & CStr(Err.Number)
End Sub
