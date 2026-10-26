' INV: vbal.5.5.1.2.1, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.1 Let-coercion between numeric types, integral value outside the destination range is Overflow
' EXPECT: ok=32767
' ERROR: 6
Public Sub Main()
    Dim i As Integer
    Dim l As Long
    l = 32767
    i = l
    Debug.Print "ok=" & CStr(i)
    l = 32768
    i = l
    Debug.Print "not reached"
End Sub
