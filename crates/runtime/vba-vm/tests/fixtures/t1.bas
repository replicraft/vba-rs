Option Explicit
Private Type Pt
    X As Long
    S As String * 3
End Type
Dim g As Long
Dim arr(1 To 3) As Long

Function Fact(ByVal n As Long) As Long
    If n <= 1 Then
        Fact = 1
    Else
        Fact = n * Fact(n - 1)
    End If
End Function

Sub Swap(ByRef a As Long, ByRef b As Long)
    Dim t As Long
    t = a: a = b: b = t
End Sub

Sub Main()
    Dim p As Pt, c As Collection, i As Long, v As Variant
    Set c = New Collection
    c.Add 5
    c.Add "x", "k"
    p.X = 3
    arr(2) = 7
    Swap arr(1), arr(2)
    On Error GoTo h
    Debug.Print Fact(5), c.Count, c(1), Err.Number
    For Each v In c
        Debug.Print v
    Next
    Err.Raise 5, "me", "boom"
    Exit Sub
h:
    Debug.Print Err.Description, Erl
    Resume Next
End Sub
