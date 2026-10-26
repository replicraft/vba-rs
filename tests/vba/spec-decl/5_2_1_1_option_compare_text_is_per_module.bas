' INV: vbal.5.2.1.1
' SOURCE: spec ms-vbal 5.2.1.1 comparison mode is per module
' EXPECT: here=False
' EXPECT: there=True
Public Sub Main()
    Debug.Print "here=" & CStr("abc" = "ABC")
    Debug.Print "there=" & CStr(Other.Same("abc", "ABC"))
End Sub
'#MODULE Other standard
Option Compare Text
Public Function Same(ByVal a As String, ByVal b As String) As Boolean
    Same = (a = b)
End Function
