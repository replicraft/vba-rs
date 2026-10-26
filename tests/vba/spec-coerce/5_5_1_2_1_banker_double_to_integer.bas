' INV: vbal.5.5.1.2.1, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.1 Let-coercion between numeric types, Double to Integer rounds half to even
' EXPECT: 0.5=0
' EXPECT: 1.5=2
' EXPECT: 2.5=2
' EXPECT: -0.5=0
' EXPECT: -1.5=-2
' EXPECT: -2.5=-2
' EXPECT: 73.5=74
' EXPECT: 74.5=74
' EXPECT: 75.5=76
' EXPECT: 76.5=76
' EXPECT: 2.25=2
' EXPECT: 2.75=3
Public Sub Main()
    Dim i As Integer
    Dim d As Double
    d = 0.5: i = d: Debug.Print "0.5=" & CStr(i)
    d = 1.5: i = d: Debug.Print "1.5=" & CStr(i)
    d = 2.5: i = d: Debug.Print "2.5=" & CStr(i)
    d = -0.5: i = d: Debug.Print "-0.5=" & CStr(i)
    d = -1.5: i = d: Debug.Print "-1.5=" & CStr(i)
    d = -2.5: i = d: Debug.Print "-2.5=" & CStr(i)
    d = 73.5: i = d: Debug.Print "73.5=" & CStr(i)
    d = 74.5: i = d: Debug.Print "74.5=" & CStr(i)
    d = 75.5: i = d: Debug.Print "75.5=" & CStr(i)
    d = 76.5: i = d: Debug.Print "76.5=" & CStr(i)
    d = 2.25: i = d: Debug.Print "2.25=" & CStr(i)
    d = 2.75: i = d: Debug.Print "2.75=" & CStr(i)
End Sub
