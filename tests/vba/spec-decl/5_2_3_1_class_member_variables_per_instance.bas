' INV: vbal.5.2.3.1
' SOURCE: spec ms-vbal 5.2.3.1 class module variables are per-instance member variables
' EXPECT: a=1 b=2
' EXPECT: shared=5
Public Sub Main()
    Dim x As New Cell
    Dim y As New Cell
    x.v = 1
    y.v = 2
    Debug.Print "a=" & CStr(x.v) & " b=" & CStr(y.v)
    Cell2.s = 5
    Debug.Print "shared=" & CStr(Cell2.s)
End Sub
'#MODULE Cell class
Public v As Long
'#MODULE Cell2 standard
Public s As Long
