' INV: vbal.5.4.2.20, stmt.raiseevent
' SOURCE: spec ms-vbal 5.4.2.20 an unhandled error in a handler prevents later handlers from running
' EXPECT: h1
' ERROR: 5
'#MODULE Src class
Public Event Fire()
Public Sub Go()
    RaiseEvent Fire
    Debug.Print "after raise"
End Sub
'#MODULE Sink class
Private WithEvents w1 As Src
Private WithEvents w2 As Src
Public Sub Run()
    Dim s As New Src
    Set w1 = s
    Set w2 = s
    s.Go
End Sub
Private Sub w1_Fire()
    Debug.Print "h1"
    Err.Raise 5
End Sub
Private Sub w2_Fire()
    Debug.Print "h2"
End Sub
'#MODULE Module1 standard
Public Sub Main()
    Dim k As New Sink
    k.Run
End Sub
