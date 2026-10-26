' INV: vbal.5.2.3.1.4
' SOURCE: spec ms-vbal 5.2.3.1.4 string length may equal 65,526
' EXPECT: 65526
Dim s As String * 65526
Public Sub Main()
    Debug.Print CStr(Len(s))
End Sub
