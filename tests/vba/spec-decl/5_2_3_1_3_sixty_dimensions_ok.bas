' INV: vbal.5.2.3.1.3
' SOURCE: spec ms-vbal 5.2.3.1.3 bounds-list holds at most 60 dim-specs
' EXPECT: last=0
' EXPECT: past=9
Dim a(0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0) As Long
Public Sub Main()
    Debug.Print "last=" & CStr(UBound(a, 60))
    On Error Resume Next
    Dim t As Long
    t = UBound(a, 61)
    Debug.Print "past=" & CStr(Err.Number)
End Sub
