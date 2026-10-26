' INV: vbal.5.2.1.2
' SOURCE: spec ms-vbal 5.2.1.2 Option Base applies only to dimensions without explicit lower bound
' EXPECT: a=1
' EXPECT: b=5
' EXPECT: m1=1 m2=0
Option Base 1
Public Sub Main()
    Dim a(3) As Long
    Dim b(5 To 9) As Long
    Dim m(2, 0 To 4) As Long
    Debug.Print "a=" & CStr(LBound(a))
    Debug.Print "b=" & CStr(LBound(b))
    Debug.Print "m1=" & CStr(LBound(m, 1)) & " m2=" & CStr(LBound(m, 2))
End Sub
