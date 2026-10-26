' INV: vbal.5.6.13.1
' SOURCE: spec ms-vbal 5.6.13.1 argument lists: positional, omitted positional, and named arguments
' EXPECT: 1-2-3
' EXPECT: 1-20-3
' EXPECT: 1-0-30
' EXPECT: 9-2-3
' EXPECT: 1-2-3
Private Function F(ByVal a As Long, Optional ByVal b As Long = 20, Optional ByVal c As Long = 3) As String
    F = a & "-" & b & "-" & c
End Function
Public Sub Main()
    Debug.Print F(1, 2, 3)
    Debug.Print F(1, , 3)
    Debug.Print F(1, c:=30, b:=0)
    Debug.Print F(c:=3, b:=2, a:=9)
    Debug.Print F(1, 2, c:=3)
End Sub
