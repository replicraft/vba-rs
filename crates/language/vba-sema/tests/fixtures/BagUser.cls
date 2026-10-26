VERSION 1.0 CLASS
BEGIN
  MultiUse = -1  'True
END
Attribute VB_Name = "BagUser"
Option Explicit
Private WithEvents bag As Bag

Public Sub Main()
    Dim b As New Bag, k As Variant, total As Double
    Set bag = b
    b.Add "x", 1
    b("y") = 2
    b.Item("z") = 3
    For Each k In b.Keys
        total = total + b(k)
    Next k
    Debug.Print b.Count, total, b!x
End Sub

Private Sub bag_Added(ByVal key As String, ByRef cancel As Boolean)
    cancel = (key = "")
End Sub
