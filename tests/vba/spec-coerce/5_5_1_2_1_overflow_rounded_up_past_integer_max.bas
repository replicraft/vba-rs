' INV: vbal.5.5.1.2.1, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.1 Let-coercion between numeric types, 32767.5 rounds to 32768 which is out of Integer range
' EXPECT: ok=32767
' ERROR: 6
Public Sub Main()
    Dim i As Integer
    Dim d As Double
    d = 32767
    i = d
    Debug.Print "ok=" & CStr(i)
    d = 32767.5
    i = d
    Debug.Print "not reached"
End Sub
