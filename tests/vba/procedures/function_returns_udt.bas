' INV: stmt.function
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT:  1  2 
' EXPECT:  2 
Type P
    X As Long
    Y As Long
End Type
Function Mk() As P
    Mk.X = 1: Mk.Y = 2
End Function
Public Sub Main()
    Dim q As P
    q = Mk()
    Debug.Print q.X; q.Y
    Debug.Print Mk().Y
End Sub
