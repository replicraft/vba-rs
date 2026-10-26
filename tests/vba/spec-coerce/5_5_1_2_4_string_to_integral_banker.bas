' INV: vbal.5.5.1.2.4, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.4 Let-coercion to and from String, string to integral type rounds the infinite-precision value with Banker's rounding
' EXPECT: 2.5=2
' EXPECT: 3.5=4
' EXPECT: -2.5=-2
' EXPECT: -3.5=-4
' EXPECT: 0.5=0
' EXPECT: 1.5=2
' EXPECT: 25e-1=2
' EXPECT: 35e-1=4
' EXPECT: 2.51=3
' EXPECT: 2.49=2
' EXPECT: byte 254.5=254
' EXPECT: long 100.5=100
Public Sub Main()
    Dim i As Integer
    Dim y As Byte
    Dim l As Long
    i = "2.5": Debug.Print "2.5=" & CStr(i)
    i = "3.5": Debug.Print "3.5=" & CStr(i)
    i = "-2.5": Debug.Print "-2.5=" & CStr(i)
    i = "-3.5": Debug.Print "-3.5=" & CStr(i)
    i = "0.5": Debug.Print "0.5=" & CStr(i)
    i = "1.5": Debug.Print "1.5=" & CStr(i)
    i = "25e-1": Debug.Print "25e-1=" & CStr(i)
    i = "35e-1": Debug.Print "35e-1=" & CStr(i)
    i = "2.51": Debug.Print "2.51=" & CStr(i)
    i = "2.49": Debug.Print "2.49=" & CStr(i)
    y = "254.5": Debug.Print "byte 254.5=" & CStr(y)
    l = "100.5": Debug.Print "long 100.5=" & CStr(l)
End Sub
