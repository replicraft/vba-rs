' INV: vbal.5.2.1.2
' SOURCE: spec ms-vbal 5.2.1.2 Option Base 1 module variable array
' EXPECT: lb=1
' EXPECT: ub=3
Option Base 1
Dim a(3) As Long
Public Sub Main()
    Debug.Print "lb=" & CStr(LBound(a))
    Debug.Print "ub=" & CStr(UBound(a))
End Sub
