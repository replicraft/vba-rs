' INV: vbal.6.1.2.11.1.35, func.split
' SOURCE: spec ms-vbal 6.1.2.11.1.35 Split returns a zero-based one-dimensional array
' EXPECT: lb=0
' EXPECT: ub=2
' EXPECT: first=a
Option Base 1
Public Sub Main()
    Dim v As Variant
    v = Split("a b c")
    Debug.Print "lb=" & CStr(LBound(v))
    Debug.Print "ub=" & CStr(UBound(v))
    Debug.Print "first=" & v(0)
End Sub
