' INV: vbal.5.2.1.1
' SOURCE: spec ms-vbal 5.2.1.1 Option Compare in a class module declaration section
' EXPECT: cls=True
' EXPECT: main=False
Public Sub Main()
    Dim c As New Cmp
    Debug.Print "cls=" & CStr(c.Same("abc", "ABC"))
    Debug.Print "main=" & CStr("abc" = "ABC")
End Sub
'#MODULE Cmp class
Option Compare Text
Public Function Same(ByVal a As String, ByVal b As String) As Boolean
    Same = (a = b)
End Function
