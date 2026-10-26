' INV: vbal.5.4.2.13
' SOURCE: spec ms-vbal 5.4.2.13 a negative selector raises error 5
' EXPECT: before
' ERROR: 5
Public Sub Main()
    Dim n As Long
    n = -1
    Debug.Print "before"
    On n GoTo L1, L2
    Debug.Print "never-fell"
L1:
L2:
End Sub
