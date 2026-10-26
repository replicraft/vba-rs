' INV: expr.new
' SOURCE: derived each New creates a separate instance
' EXPECT:  1  2 
' EXPECT: False
'#MODULE Box class
Public V As Long
'#MODULE Module1 standard
Public Sub Main()
    Dim a As New Box, b As New Box
    a.V = 1: b.V = 2
    Debug.Print a.V; b.V
    Debug.Print a Is b
End Sub
