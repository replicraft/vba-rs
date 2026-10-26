' INV: vbal.5.2.1.2
' SOURCE: spec ms-vbal 5.2.1.2 Option Base applies to ReDim dim-spec
' EXPECT: lb=1
' EXPECT: ub=4
Option Base 1
Public Sub Main()
    Dim a() As Long
    ReDim a(4)
    Debug.Print "lb=" & CStr(LBound(a))
    Debug.Print "ub=" & CStr(UBound(a))
End Sub
