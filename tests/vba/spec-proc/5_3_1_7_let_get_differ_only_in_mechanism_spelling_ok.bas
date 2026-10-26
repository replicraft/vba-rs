' INV: vbal.5.3.1.7
' SOURCE: spec ms-vbal 5.3.1.7 property parameters may differ in implicit vs explicit mechanism
' EXPECT: v=4
'#MODULE Cls class
Private x As Long
Public Property Get P(i As Long) As Long
    P = x
End Property
Public Property Let P(ByRef i As Long, ByVal v As Long)
    x = v
End Property
'#MODULE Module1 standard
Public Sub Main()
    Dim c As New Cls
    Dim k As Long
    c.P(k) = 4
    Debug.Print "v=" & CStr(c.P(k))
End Sub
