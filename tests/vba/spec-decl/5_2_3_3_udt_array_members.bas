' INV: vbal.5.2.3.3
' SOURCE: spec ms-vbal 5.2.3.3 UDT array members: fixed with bounds-list, resizable without
' EXPECT: 1 3
' EXPECT: 0 2
' EXPECT: ub=4
Option Base 1
Private Type Rec
    f(3) As Long
    g(0 To 2) As Long
    r() As Long
End Type
Public Sub Main()
    Dim x As Rec
    Debug.Print CStr(LBound(x.f)) & " " & CStr(UBound(x.f))
    Debug.Print CStr(LBound(x.g)) & " " & CStr(UBound(x.g))
    ReDim x.r(4)
    Debug.Print "ub=" & CStr(UBound(x.r))
End Sub
