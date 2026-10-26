' INV: vbal.2.3
' SOURCE: spec ms-vbal 2.3 variables, object-extent variables: each instance has its own variables created with default values
' EXPECT: 0 0
' EXPECT: 2 0
' EXPECT: 2 5
'#MODULE Counter class
Public N As Long
'#MODULE Module1 standard
Public Sub Main()
    Dim a As New Counter, b As New Counter
    Debug.Print a.N & " " & b.N
    a.N = 2
    Debug.Print a.N & " " & b.N
    b.N = 5
    Debug.Print a.N & " " & b.N
End Sub
