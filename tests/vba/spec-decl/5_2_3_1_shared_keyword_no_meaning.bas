' INV: vbal.5.2.3.1
' SOURCE: spec ms-vbal 5.2.3.1 optional Shared keyword has no meaning
' EXPECT: v=5
Private Shared a As Long
Public Shared b As Long
Public Sub Main()
    a = 2
    b = 3
    Debug.Print "v=" & CStr(a + b)
End Sub
