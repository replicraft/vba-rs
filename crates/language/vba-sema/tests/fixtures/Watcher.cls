VERSION 1.0 CLASS
BEGIN
  MultiUse = -1  'True
END
Attribute VB_Name = "Watcher"
Option Explicit

Private WithEvents target As Disc
Private log As Collection

Public Sub Attach(ByVal c As Disc)
    Set target = c
    If log Is Nothing Then Set log = New Collection
End Sub

Private Sub target_Resized(ByVal newRadius As Double)
    log.Add "radius=" & newRadius
End Sub

Public Property Get Entries() As Long
    Entries = log.Count
End Property

Public Property Get Item(ByVal index As Long) As String
Attribute Item.VB_UserMemId = 0
    Item = log(index)
End Property
