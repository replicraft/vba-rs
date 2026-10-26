Attribute VB_Name = "Tour"
Option Explicit
Public Enum Kind
    kA = 1
    kB
    kC
End Enum
Type Pt
    x As Long
    y As Long
End Type
Private Sub TakeArr(a() As Long)
End Sub
Private Sub TakePts(p() As Pt, ByRef n As Long)
End Sub
Private Function Pick(Optional ByVal k As Kind = kB) As Kind
    Pick = k
End Function

Sub Main()
    Dim i As Integer, f As Single, b As Byte, s As String * 10, t As String
    Dim d As Date, c As Currency, v As Variant, a(10) As Integer, o As Object
    Dim e As Kind, arr() As Long, pts(1 To 3) As Pt, n As Long, bool As Boolean
    Dim w As Variant, coll As Collection, i2&, s2$, d2#
    i = i + 1
    f = 1 / 3
    b = 255
    b = b + 1
    s = "a"
    t = s
    d = Now
    d = d + 1
    d = #1/1/2000#
    d = DateSerial(2000, 1, 1)
    c = 1.5@
    v = Array()
    ReDim v(3)
    a(1) = a(2) + 1
    ReDim arr(1 To 5)
    TakeArr arr
    TakeArr arr()
    TakePts pts, n
    Set o = Nothing
    If o Is Nothing Then Debug.Print "nothing"
    e = kA
    If e = 1 Then e = kC
    For e = kA To kC
        Debug.Print e
    Next e
    i2& = 1
    s2$ = "x"
    d2# = 2.5
    n = CLng(3.5) + Int(2.2) + Abs(-3)
    bool = Not bool
    bool = bool Xor True
    bool = (i > 0) And (n < 5) Or (s2 = "x")
    w = Split("a b")(0)
    Set coll = New Collection
    coll.Add Item:=w, Key:="k"
    If t < s2 Then Debug.Print TypeName(w)
    e = Pick()
    e = Pick(kC)
    Open "f" For Binary As #1
    Get #1, , n
    Put #1, 2, n
    Close #1
    n = e + i2
    t = s & i & f & c & d
    With coll
        .Add 1
        n = .Count
    End With
    n = pts(1).x + pts(2).y
    pts(3).x = pts(1).x
    t = Left(s, 2) & Mid(s, 2)
    Mid(t, 1, 1) = "z"
    n = Len(t) * 2 \ 3 Mod 4 ^ 2
    v = n > 3
    If v Then n = 0
    v = Null
    If IsNull(v) Then n = -1
    n = Switch(n > 0, 1, n < 0, 2)
    Randomize
    n = Int(Rnd * 10)
End Sub
