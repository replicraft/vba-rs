' INV: vbal.5.5.1.2.3, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.3 Let-coercion to and from Date, numeric/Boolean to Date via the standard Double representation
' EXPECT: dbl=1.5
' EXPECT: int=5
' EXPECT: bool=-1
' EXPECT: neg=-2.75
' EXPECT: max=2958465
' EXPECT: min=-657434
' EXPECT: empty=0
' EXPECT: long=100000
' EXPECT: typename=Date
Public Sub Main()
    Dim d As Date
    Dim i As Integer
    Dim b As Boolean
    Dim l As Long
    Dim v As Variant
    d = 1.5: Debug.Print "dbl=" & CStr(CDbl(d))
    i = 5: d = i: Debug.Print "int=" & CStr(CDbl(d))
    b = True: d = b: Debug.Print "bool=" & CStr(CDbl(d))
    d = -2.75: Debug.Print "neg=" & CStr(CDbl(d))
    d = 2958465: Debug.Print "max=" & CStr(CDbl(d))
    d = -657434: Debug.Print "min=" & CStr(CDbl(d))
    d = v: Debug.Print "empty=" & CStr(CDbl(d))
    l = 100000: d = l: Debug.Print "long=" & CStr(CDbl(d))
    Debug.Print "typename=" & TypeName(d)
End Sub
