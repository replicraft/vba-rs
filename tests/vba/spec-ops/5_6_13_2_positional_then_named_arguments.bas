' INV: vbal.5.6.13.2
' SOURCE: derived 5.6.13.2 positional arguments fill the parameters in order and named arguments fill the parameter they name; unfilled optional parameters take their defaults
' METHOD: by hand from the signature F(a, b=10, c=20)
' EXPECT: 1,10,20
' EXPECT: 1,2,20
' EXPECT: 1,10,3
' EXPECT: 1,10,3
' EXPECT: 1,10,3
' EXPECT: 5,10,20

Private Function F(a As Long, Optional b As Long = 10, Optional c As Long = 20) As String
    F = a & "," & b & "," & c
End Function
Public Sub Main()
    Debug.Print F(1)
    Debug.Print F(1, 2)
    Debug.Print F(1, c:=3)
    Debug.Print F(c:=3, a:=1)
    Debug.Print F(1, , 3)
    Debug.Print F(a:=5)
End Sub
