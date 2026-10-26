' INV: vbal.5.2.3.3
' SOURCE: spec ms-vbal 5.2.3.3 public UDT accessible within the project
' EXPECT: 5 6
Public Sub Main()
    Dim p As Pair
    p.a = 5
    p.b = 6
    Debug.Print CStr(p.a) & " " & CStr(p.b)
End Sub
'#MODULE Types standard
Public Type Pair
    a As Long
    b As Long
End Type
