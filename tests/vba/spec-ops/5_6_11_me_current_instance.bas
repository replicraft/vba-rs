' INV: vbal.5.6.11
' SOURCE: spec ms-vbal 5.6.11 Me is the current instance of the enclosing class module
' EXPECT: True
' EXPECT: 5
' EXPECT: Counter
' EXPECT: True
'#MODULE Counter class
Public N As Long
Public Function Self() As Counter
    Set Self = Me
End Function
Public Sub SetN(ByVal v As Long)
    Me.N = v
End Sub
Public Function Who() As String
    Who = TypeName(Me)
End Function
'#MODULE Module1 standard
Public Sub Main()
    Dim c As New Counter
    Debug.Print CStr(c.Self Is c)
    c.SetN 5
    Debug.Print CStr(c.N)
    Debug.Print c.Who
    Dim d As New Counter
    Debug.Print CStr(Not (d.Self Is c))
End Sub
