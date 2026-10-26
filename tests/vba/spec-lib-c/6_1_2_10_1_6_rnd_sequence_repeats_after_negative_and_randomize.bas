' INV: vbal.6.1.2.10.1.6, func.rnd, stmt.randomize
' SOURCE: spec ms-vbal 6.1.2.10.1.6 Rnd negative then Randomize numeric repeats the sequence; Rnd and Rnd(positive) are both "next"
' EXPECT: True
' EXPECT: True
Public Sub Main()
    Dim a(1 To 5) As Single, b(1 To 5) As Single, i As Long, same As Boolean
    Rnd -1
    Randomize 42
    For i = 1 To 5: a(i) = Rnd: Next i
    Rnd -1
    Randomize 42
    For i = 1 To 5: b(i) = Rnd: Next i
    same = True
    For i = 1 To 5
        If a(i) <> b(i) Then same = False
    Next i
    Debug.Print same
    ' Rnd(positive) continues the same sequence as Rnd without argument
    Rnd -1
    Randomize 42
    same = True
    For i = 1 To 5
        If Rnd(1) <> a(i) Then same = False
    Next i
    Debug.Print same
End Sub
