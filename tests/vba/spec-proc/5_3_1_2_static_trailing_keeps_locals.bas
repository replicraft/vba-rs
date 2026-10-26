' INV: vbal.5.3.1.2
' SOURCE: spec ms-vbal 5.3.1.2 trailing Static gives module extent to locals
' XFAIL: wrong not implemented: trailing Static in a procedure header
' EXPECT: 1
' EXPECT: 2
' EXPECT: 3
Function Tick() As Long Static
    Dim n As Long
    n = n + 1
    Tick = n
End Function
Public Sub Main()
    Debug.Print CStr(Tick())
    Debug.Print CStr(Tick())
    Debug.Print CStr(Tick())
End Sub
