' INV: vbal.5.5.1.2.4, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.4 Let-coercion to and from String, a number string out of the destination range is Overflow
' EXPECT: ok=32767
' ERROR: 6
Public Sub Main()
    Dim i As Integer
    i = "32767"
    Debug.Print "ok=" & CStr(i)
    i = "40000"
    Debug.Print "not reached"
End Sub
