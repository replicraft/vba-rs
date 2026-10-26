' INV: vbal.5.4.2.20, stmt.raiseevent
' SOURCE: spec ms-vbal 5.4.2.20 handlers run in WithEvents initialisation order; reassigning a variable moves it last
' EXPECT: a b
' EXPECT: b a
'#MODULE Src class
Public Event Fire()
Public Sub Go()
    RaiseEvent Fire
End Sub
'#MODULE Sink class
Public Trail As String
Private WithEvents wa As Src
Private WithEvents wb As Src
Public Sub Attach(ByVal s As Src)
    Set wa = s
    Set wb = s
End Sub
Public Sub Reattach(ByVal s As Src)
    Set wa = s
End Sub
Private Sub wa_Fire()
    Trail = Trail & "a "
End Sub
Private Sub wb_Fire()
    Trail = Trail & "b "
End Sub
'#MODULE Module1 standard
Public Sub Main()
    Dim s As New Src
    Dim k As New Sink
    k.Attach s
    s.Go
    Debug.Print Trim$(k.Trail)
    k.Trail = ""
    k.Reattach s
    s.Go
    Debug.Print Trim$(k.Trail)
End Sub
