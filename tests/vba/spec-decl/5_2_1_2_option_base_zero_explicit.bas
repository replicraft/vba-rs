' INV: vbal.5.2.1.2
' SOURCE: spec ms-vbal 5.2.1.2 Option Base 0
' EXPECT: lb=0
Option Base 0
Public Sub Main()
    Dim a(3) As Long
    Debug.Print "lb=" & CStr(LBound(a))
End Sub
