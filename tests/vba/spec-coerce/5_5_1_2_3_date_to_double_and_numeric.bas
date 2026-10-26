' INV: vbal.5.5.1.2.3, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.3 Let-coercion to and from Date, Date to numeric uses the standard Double representation
' EXPECT: dbl=2.25
' EXPECT: int=2
' EXPECT: int=4
' EXPECT: long=-3
' EXPECT: bool=True
' EXPECT: boolzero=False
' EXPECT: copy=2.25
Public Sub Main()
    Dim d As Date
    Dim x As Double
    Dim i As Integer
    Dim l As Long
    Dim b As Boolean
    Dim d2 As Date
    d = 2.25
    x = d
    Debug.Print "dbl=" & CStr(x)
    d = 2.5
    i = d
    Debug.Print "int=" & CStr(i)
    d = 3.5
    i = d
    Debug.Print "int=" & CStr(i)
    d = -3.25
    l = d
    Debug.Print "long=" & CStr(l)
    d = 0.25
    b = d
    Debug.Print "bool=" & CStr(b)
    d = 0
    b = d
    Debug.Print "boolzero=" & CStr(b)
    d = 2.25
    d2 = d
    Debug.Print "copy=" & CStr(CDbl(d2))
End Sub
