' INV: vbal.5.6.9.7, op.is
' SOURCE: spec ms-vbal 5.6.9.7 Is operator reference equality
' EXPECT: True
' EXPECT: False
' EXPECT: True
' EXPECT: False
' EXPECT: True
' EXPECT: Boolean
'#MODULE Thing class
Public X As Long
'#MODULE Module1 standard
Public Sub Main()
    Dim a As New Thing, b As New Thing, c As Thing, o As Object, n As Object
    Set c = a
    Debug.Print CStr(a Is c)
    Debug.Print CStr(a Is b)
    Set o = a
    Debug.Print CStr(o Is a)
    Debug.Print CStr(n Is a)
    Debug.Print CStr(n Is Nothing)
    Debug.Print TypeName(a Is b)
End Sub
