' INV: vbal.5.3.1.7
' SOURCE: spec ms-vbal 5.3.1.7 value parameter of Property Let has ByVal semantics even if ByRef is written
' EXPECT: inside=99
' EXPECT: caller=1
'#MODULE Cls class
Public Property Let P(ByRef v As Long)
    v = 99
    Debug.Print "inside=" & CStr(v)
End Property
'#MODULE Module1 standard
Public Sub Main()
    Dim c As New Cls
    Dim n As Long
    n = 1
    c.P = n
    Debug.Print "caller=" & CStr(n)
End Sub
